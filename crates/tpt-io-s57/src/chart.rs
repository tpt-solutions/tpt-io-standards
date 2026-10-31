//! S-57 chart content on top of the ISO 8211 framing: the dataset parameters, the features with
//! their attributes, and the coordinates their spatial references resolve to.
//!
//! The file must be self-contained: every vector record a feature refers to must be in the same
//! file. Update files that depend on a base file are rejected rather than partly read.

use alloc::{collections::BTreeMap, format, string::String, string::ToString, vec::Vec};

use crate::error::S57Error;
use crate::iso8211::{self, Subfield, Value};

/// Record name of an edge vector record.
const RCNM_EDGE: u8 = 130;

/// A coordinate in degrees, with a depth in metres for 3D soundings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// Latitude in degrees.
    pub latitude: f64,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Depth in metres, from `SG3D`; `None` for 2D coordinates.
    pub depth: Option<f64>,
}

/// The geometric primitive of a feature (`PRIM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Primitive {
    /// A point feature (1).
    Point,
    /// A line feature (2).
    Line,
    /// An area feature (3).
    Area,
    /// Any other primitive code.
    Other(u8),
}

/// One feature attribute (`ATTF`): the attribute code and its value as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The attribute label code (`ATTL`).
    pub code: u16,
    /// The attribute value (`ATVL`), as text.
    pub value: String,
}

/// The geometry a feature resolves to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Geometry {
    /// The feature has no spatial reference.
    None,
    /// Point positions, from its connected or isolated nodes.
    Points(Vec<Position>),
    /// One position chain per edge reference, in the order the feature lists them. Each chain
    /// runs from its begin node to its end node, with the edge's interior points between, and
    /// is reversed when the reference's orientation is reverse. Joining the chains into rings
    /// or lines is left to the caller.
    Curves(Vec<Vec<Position>>),
}

/// One feature object.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// The record identifier (`RCID`) within the file.
    pub id: u32,
    /// The object class code (`OBJL`). The class names are in the S-57 object catalogue.
    pub object_class: u16,
    /// The geometric primitive (`PRIM`).
    pub primitive: Primitive,
    /// The attributes, in file order.
    pub attributes: Vec<Attribute>,
    /// The resolved geometry.
    pub geometry: Geometry,
}

/// A decoded S-57 exchange file.
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    /// The dataset name (`DSNM`).
    pub name: String,
    /// The edition number (`EDTN`).
    pub edition: String,
    /// The coordinate multiplying factor (`COMF`): stored coordinates are degrees times this.
    pub coordinate_multiplier: f64,
    /// The sounding multiplying factor (`SOMF`): stored depths are metres times this.
    pub sounding_multiplier: f64,
    /// The feature objects, in file order.
    pub features: Vec<Feature>,
}

/// One decoded field of a data record, keyed by its directory tag.
struct Entry {
    tag: String,
    subfields: Vec<Subfield>,
}

/// A vector record: a node or edge, with its own coordinates and its end references.
struct Vector {
    positions: Vec<Position>,
    ends: Vec<(u8, u32)>,
}

/// Parses an S-57 exchange file into a [`Dataset`].
pub fn parse(input: &[u8]) -> Result<Dataset, S57Error> {
    let records = iso8211::read_records(input)?;
    let (ddr, data) = records.split_first().ok_or_else(|| invalid("no data descriptive record"))?;
    let dictionary = iso8211::dictionary(ddr)?;

    let mut rows = Vec::with_capacity(data.len());
    for record in data {
        let mut row = Vec::new();
        for field in &record.fields {
            // Fields the dictionary does not define are skipped rather than rejected.
            if let Some(def) = dictionary.get(&field.tag) {
                row.push(Entry {
                    tag: field.tag.clone(),
                    subfields: iso8211::decode(def, &field.data)?,
                });
            }
        }
        rows.push(row);
    }

    let mut name = String::new();
    let mut edition = String::new();
    let mut comf = None;
    let mut somf = None;
    for row in &rows {
        for entry in row {
            match entry.tag.as_str() {
                "DSID" => {
                    name = text(need(&entry.subfields, "DSNM")?)?;
                    edition = text(need(&entry.subfields, "EDTN")?)?;
                }
                "DSPM" => {
                    comf = Some(number(need(&entry.subfields, "COMF")?)?);
                    somf = Some(number(need(&entry.subfields, "SOMF")?)?);
                }
                _ => {}
            }
        }
    }
    let coordinate_multiplier = comf
        .filter(|&v| v > 0.0)
        .ok_or_else(|| invalid("dataset parameters (DSPM) give no positive COMF"))?;
    let sounding_multiplier = somf
        .filter(|&v| v > 0.0)
        .ok_or_else(|| invalid("dataset parameters (DSPM) give no positive SOMF"))?;
    let scale = Scale { coordinate: coordinate_multiplier, sounding: sounding_multiplier };

    // Vector records first: features refer to them by record name and identifier.
    let mut vectors: BTreeMap<(u8, u32), Vector> = BTreeMap::new();
    for row in &rows {
        if let Some(vrid) = find(row, "VRID") {
            let key = record_key(&vrid.subfields)?;
            let vector = Vector { positions: coordinates(row, scale)?, ends: vector_ends(row)? };
            vectors.insert(key, vector);
        }
    }

    let mut features = Vec::new();
    for row in &rows {
        if let Some(frid) = find(row, "FRID") {
            features.push(feature(row, frid, &vectors, scale)?);
        }
    }

    Ok(Dataset { name, edition, coordinate_multiplier, sounding_multiplier, features })
}

#[derive(Clone, Copy)]
struct Scale {
    coordinate: f64,
    sounding: f64,
}

fn feature(
    row: &[Entry],
    frid: &Entry,
    vectors: &BTreeMap<(u8, u32), Vector>,
    scale: Scale,
) -> Result<Feature, S57Error> {
    let id = record_key(&frid.subfields)?.1;
    let object_class = u16::try_from(integer(need(&frid.subfields, "OBJL")?)?)
        .map_err(|_| invalid("object class OBJL does not fit in 16 bits"))?;
    let primitive = match integer(need(&frid.subfields, "PRIM")?)? {
        1 => Primitive::Point,
        2 => Primitive::Line,
        3 => Primitive::Area,
        other => Primitive::Other(u8::try_from(other).unwrap_or(u8::MAX)),
    };

    let mut attributes = Vec::new();
    for entry in row.iter().filter(|e| e.tag == "ATTF") {
        for group in groups(&entry.subfields, "ATTL") {
            let code = u16::try_from(integer(need(group, "ATTL")?)?)
                .map_err(|_| invalid("attribute label ATTL does not fit in 16 bits"))?;
            attributes.push(Attribute { code, value: text(need(group, "ATVL")?)? });
        }
    }

    // Spatial references: each FSPT entry names a vector record and its orientation.
    let mut references = Vec::new();
    for entry in row.iter().filter(|e| e.tag == "FSPT") {
        for group in groups(&entry.subfields, "NAME") {
            let key = name_ref(need(group, "NAME")?)?;
            let reversed = value(group, "ORNT").and_then(as_u64) == Some(2);
            references.push((key, reversed));
        }
    }
    let own = coordinates(row, scale)?;

    let geometry = if references.is_empty() && own.is_empty() {
        Geometry::None
    } else if primitive == Primitive::Point {
        let mut points = own;
        for &(key, _) in &references {
            points.extend_from_slice(&vector(vectors, key, id)?.positions);
        }
        Geometry::Points(points)
    } else {
        let mut curves = Vec::new();
        for &(key, reversed) in &references {
            // Line and area features reference edges; node references do not form curves.
            if key.0 != RCNM_EDGE {
                continue;
            }
            let mut chain = edge_chain(vectors, key, id)?;
            if reversed {
                chain.reverse();
            }
            curves.push(chain);
        }
        Geometry::Curves(curves)
    };

    Ok(Feature { id, object_class, primitive, attributes, geometry })
}

/// An edge's chain: its begin node, its interior points, then its end node.
fn edge_chain(
    vectors: &BTreeMap<(u8, u32), Vector>,
    key: (u8, u32),
    feature: u32,
) -> Result<Vec<Position>, S57Error> {
    let edge = vector(vectors, key, feature)?;
    let mut chain = Vec::with_capacity(edge.positions.len() + 2);
    if let Some(&begin) = edge.ends.first() {
        chain.push(node_position(vectors, begin, feature)?);
    }
    chain.extend_from_slice(&edge.positions);
    if let Some(&end) = edge.ends.get(1) {
        chain.push(node_position(vectors, end, feature)?);
    }
    Ok(chain)
}

fn node_position(
    vectors: &BTreeMap<(u8, u32), Vector>,
    key: (u8, u32),
    feature: u32,
) -> Result<Position, S57Error> {
    let node = vector(vectors, key, feature)?;
    node.positions
        .first()
        .copied()
        .ok_or_else(|| invalid(&format!("node {} of feature {feature} has no coordinates", key.1)))
}

fn vector(
    vectors: &BTreeMap<(u8, u32), Vector>,
    key: (u8, u32),
    feature: u32,
) -> Result<&Vector, S57Error> {
    vectors.get(&key).ok_or_else(|| {
        invalid(&format!(
            "feature {feature} refers to vector record {} (name {}) not in the file",
            key.1, key.0
        ))
    })
}

/// Reads the `SG2D` or `SG3D` coordinates of one record, in file order.
fn coordinates(row: &[Entry], scale: Scale) -> Result<Vec<Position>, S57Error> {
    let mut out = Vec::new();
    for entry in row.iter().filter(|e| e.tag == "SG2D" || e.tag == "SG3D") {
        let three_d = entry.tag == "SG3D";
        for group in groups(&entry.subfields, "YCOO") {
            let latitude = integer_signed(need(group, "YCOO")?)? as f64 / scale.coordinate;
            let longitude = integer_signed(need(group, "XCOO")?)? as f64 / scale.coordinate;
            let depth = if three_d {
                Some(integer_signed(need(group, "VE3D")?)? as f64 / scale.sounding)
            } else {
                None
            };
            out.push(Position { latitude, longitude, depth });
        }
    }
    Ok(out)
}

/// The end references of a vector record (`VRPT`), begin node first.
fn vector_ends(row: &[Entry]) -> Result<Vec<(u8, u32)>, S57Error> {
    let mut ends = Vec::new();
    for entry in row.iter().filter(|e| e.tag == "VRPT") {
        for group in groups(&entry.subfields, "NAME") {
            ends.push(name_ref(need(group, "NAME")?)?);
        }
    }
    Ok(ends)
}

/// Reads a 5-byte record name: the record name code, then the record identifier (LE).
fn name_ref(value: &Value) -> Result<(u8, u32), S57Error> {
    match value {
        Value::Bytes(b) if b.len() == 5 => Ok((b[0], u32::from_le_bytes([b[1], b[2], b[3], b[4]]))),
        _ => Err(invalid("NAME is not a 5-byte record reference")),
    }
}

fn record_key(subfields: &[Subfield]) -> Result<(u8, u32), S57Error> {
    let rcnm = u8::try_from(integer(need(subfields, "RCNM")?)?)
        .map_err(|_| invalid("record name RCNM does not fit in a byte"))?;
    let rcid = u32::try_from(integer(need(subfields, "RCID")?)?)
        .map_err(|_| invalid("record identifier RCID does not fit in 32 bits"))?;
    Ok((rcnm, rcid))
}

/// Splits repeated subfields into groups, each starting at a `start` label.
fn groups<'a>(subfields: &'a [Subfield], start: &str) -> Vec<&'a [Subfield]> {
    let starts: Vec<usize> =
        subfields.iter().enumerate().filter(|(_, s)| s.label == start).map(|(i, _)| i).collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &from)| {
            let to = starts.get(k + 1).copied().unwrap_or(subfields.len());
            &subfields[from..to]
        })
        .collect()
}

fn find<'a>(row: &'a [Entry], tag: &str) -> Option<&'a Entry> {
    row.iter().find(|e| e.tag == tag)
}

fn value<'a>(subfields: &'a [Subfield], label: &str) -> Option<&'a Value> {
    subfields.iter().find(|s| s.label == label).map(|s| &s.value)
}

fn need<'a>(subfields: &'a [Subfield], label: &str) -> Result<&'a Value, S57Error> {
    value(subfields, label).ok_or_else(|| invalid(&format!("missing subfield {label}")))
}

fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Unsigned(u) => Some(*u),
        Value::Signed(i) => u64::try_from(*i).ok(),
        Value::Text(s) => s.trim().parse().ok(),
        Value::Bytes(_) => None,
    }
}

fn integer(value: &Value) -> Result<u64, S57Error> {
    as_u64(value).ok_or_else(|| invalid("expected an unsigned integer subfield"))
}

fn integer_signed(value: &Value) -> Result<i64, S57Error> {
    match value {
        Value::Signed(i) => Ok(*i),
        Value::Unsigned(u) => i64::try_from(*u).map_err(|_| invalid("integer subfield too large")),
        Value::Text(s) => s.trim().parse().map_err(|_| invalid("expected an integer subfield")),
        Value::Bytes(_) => Err(invalid("expected an integer subfield")),
    }
}

fn number(value: &Value) -> Result<f64, S57Error> {
    integer_signed(value).map(|v| v as f64)
}

fn text(value: &Value) -> Result<String, S57Error> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        Value::Unsigned(u) => Ok(u.to_string()),
        Value::Signed(i) => Ok(i.to_string()),
        Value::Bytes(_) => Err(invalid("expected a text subfield")),
    }
}

fn invalid(message: &str) -> S57Error {
    S57Error::Invalid(String::from(message))
}
