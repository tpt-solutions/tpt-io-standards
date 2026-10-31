//! GeoPackage: an SQLite database that holds vector feature tables (OGC 12-128r19).
//!
//! [`layers`] lists the feature tables and their geometry columns. [`read_features`] reads one feature table into a
//! [`FeatureCollection`]: each row is a feature, its geometry comes from the GeoPackage binary geometry (a `GP`
//! header followed by WKB), its other columns become properties, and its rowid is the feature id.
//!
//! The database is read through [`crate::sqlite`], which covers the parts of SQLite that feature tables use. Tile
//! and raster tables, extensions, and the envelope in the geometry header are not interpreted.
//!
//! Spec access: GeoPackage is an OGC standard published openly. No specification text is copied into this crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::GeoError;
use crate::geojson::{Feature, FeatureCollection, FeatureId, Geometry};
use crate::json::Json;
use crate::sqlite::{int, text, Database, Value};
use crate::wkb::parse_wkb;

/// A feature table and its geometry column, from `gpkg_geometry_columns`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// The table name.
    pub table: String,
    /// The geometry column name.
    pub geometry_column: String,
    /// The geometry type name, such as `POINT` or `MULTIPOLYGON`.
    pub geometry_type: String,
    /// The spatial reference system id.
    pub srs_id: i64,
}

/// The feature tables in a GeoPackage.
pub fn layers(data: &[u8]) -> Result<Vec<Layer>, GeoError> {
    let db = Database::open(data)?;
    let root = db.table_root("gpkg_geometry_columns")?;
    let mut out = Vec::new();
    for row in db.rows(root)? {
        let v = &row.values;
        if v.len() < 4 {
            continue;
        }
        out.push(Layer {
            table: text(&v[0]),
            geometry_column: text(&v[1]),
            geometry_type: text(&v[2]),
            srs_id: int(&v[3]).unwrap_or(0),
        });
    }
    Ok(out)
}

/// Read a feature table into a feature collection, one feature per row in rowid order.
pub fn read_features(data: &[u8], table: &str) -> Result<FeatureCollection, GeoError> {
    let db = Database::open(data)?;
    let schema = db.schema()?;
    let entry = schema
        .iter()
        .find(|s| s.kind == "table" && s.name == table)
        .ok_or_else(|| GeoError::custom(0, alloc::format!("no table named {table}")))?;
    let columns = column_names(&entry.sql);
    let geometry_column = geometry_column(&db, table)?;
    let geometry_index = columns.iter().position(|c| *c == geometry_column).ok_or_else(|| {
        GeoError::custom(0, alloc::format!("table {table} has no column {geometry_column}"))
    })?;

    let mut features = Vec::new();
    for row in db.rows(entry.root_page)? {
        let geometry = match row.values.get(geometry_index) {
            Some(Value::Blob(blob)) => gpkg_geometry(blob)?,
            _ => None,
        };
        let mut props = Vec::new();
        for (i, name) in columns.iter().enumerate() {
            if i == geometry_index {
                continue;
            }
            let value = row.values.get(i).cloned().unwrap_or(Value::Null);
            // An INTEGER PRIMARY KEY column is stored as NULL; its value is the rowid.
            let json = match value {
                Value::Null if is_rowid_alias(&entry.sql, name) => Json::Number(row.rowid as f64),
                Value::Null => Json::Null,
                Value::Int(i) => Json::Number(i as f64),
                Value::Real(r) => Json::Number(r),
                Value::Text(s) => Json::String(s),
                Value::Blob(_) => Json::Null,
            };
            props.push((name.clone(), json));
        }
        features.push(Feature {
            geometry: geometry.map(|geometry| Geometry {
                geometry: geometry.geometry,
                bbox: None,
                foreign_members: Vec::new(),
            }),
            properties: Json::Object(props),
            id: Some(FeatureId::Number(row.rowid as f64)),
            bbox: None,
            foreign_members: Vec::new(),
        });
    }
    Ok(FeatureCollection { features, bbox: None, foreign_members: Vec::new() })
}

/// Decode a GeoPackage geometry blob: the `GP` header (magic, version, flags and SRS id, then an optional envelope),
/// then WKB. An empty geometry decodes to `None`.
pub fn gpkg_geometry(blob: &[u8]) -> Result<Option<Geometry>, GeoError> {
    if blob.len() < 8 || &blob[0..2] != b"GP" {
        return Err(GeoError::custom(0, "not a GeoPackage geometry (no GP header)"));
    }
    let flags = blob[3];
    if flags & 0x10 != 0 {
        return Ok(None);
    }
    let envelope = match (flags >> 1) & 0x07 {
        0 => 0,
        1 => 32,
        2 | 3 => 48,
        4 => 64,
        _ => return Err(GeoError::custom(3, "invalid GeoPackage envelope indicator")),
    };
    let at = 8 + envelope;
    let wkb = blob.get(at..).ok_or_else(|| GeoError::eof(at))?;
    Ok(Some(parse_wkb(wkb)?))
}

/// The geometry column of a feature table, from `gpkg_geometry_columns`.
fn geometry_column(db: &Database<'_>, table: &str) -> Result<String, GeoError> {
    let root = db.table_root("gpkg_geometry_columns")?;
    for row in db.rows(root)? {
        if row.values.len() >= 2 && text(&row.values[0]) == table {
            return Ok(text(&row.values[1]));
        }
    }
    Err(GeoError::custom(0, alloc::format!("table {table} is not a GeoPackage feature table")))
}

/// The column names of a `CREATE TABLE` statement, in order. Table-level constraints are skipped.
fn column_names(sql: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (sql.find('('), sql.rfind(')')) else {
        return Vec::new();
    };
    let body = &sql[open + 1..close];
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in body.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .filter(|p| {
            let first = p.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
            !matches!(first.as_str(), "PRIMARY" | "UNIQUE" | "CHECK" | "CONSTRAINT" | "FOREIGN")
        })
        .map(|p| {
            let name = p.split_whitespace().next().unwrap_or("");
            name.trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']').to_string()
        })
        .collect()
}

/// Whether `name` is declared `INTEGER PRIMARY KEY` in the table's `CREATE` statement.
fn is_rowid_alias(sql: &str, name: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    lower.contains(&alloc::format!("{} integer primary key", name.to_ascii_lowercase()))
}
