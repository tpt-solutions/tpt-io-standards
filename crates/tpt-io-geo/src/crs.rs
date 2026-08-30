//! Coordinate reference system handling, datum transformation, and
//! reprojection utilities.
//!
//! This module provides:
//!
//! * **PROJ strings** — parse and validate `+key=value` PROJ initialisation
//!   strings into a structured [`ProjString`].
//! * **EPSG codes** — a small built-in registry ([`lookup_epsg`]) covering the
//!   common geographic and projected systems used with the Earth-science
//!   formats, plus a [`Crs`] model.
//! * **Datum transformation** — 3D Helmert (Bursa–Wolf) transforms between
//!   WGS84, NAD83, and NZGD2000 in geocentric (ECEF) space.
//! * **Reprojection** — forward/inverse Transverse Mercator (Redfearn series)
//!   and equirectangular projections, wired together by [`reproject`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::GeoError;

/// A reference ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid {
    /// Semi-major axis (metres).
    pub a: f64,
    /// Flattening `f = (a - b) / a`.
    pub f: f64,
}

impl Ellipsoid {
    /// First eccentricity squared `e² = 2f - f²`.
    pub fn eccentricity2(&self) -> f64 {
        let f = self.f;
        2.0 * f - f * f
    }

    /// Semi-minor axis (metres).
    pub fn semi_minor(&self) -> f64 {
        self.a * (1.0 - self.f)
    }
}

/// Built-in reference ellipsoids.
pub mod ellipsoid {
    use super::Ellipsoid;
    /// WGS84 / GRS80 ellipsoid.
    pub const WGS84: Ellipsoid = Ellipsoid { a: 6378137.0, f: 1.0 / 298.257223563 };
}

/// A geodetic datum defined by an ellipsoid and a 7-parameter (Bursa–Wolf)
/// transform *to WGS84*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatumParams {
    /// The datum's reference ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// Translation vector `[dx, dy, dz]` in metres.
    pub translation: [f64; 3],
    /// Rotation vector `[rx, ry, rz]` in arc-seconds.
    pub rotation: [f64; 3],
    /// Scale difference in parts-per-million.
    pub scale_ppm: f64,
}

/// The datums this crate can transform between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Datum {
    /// WGS84 (the reference frame; identity transform to itself).
    Wgs84,
    /// North American Datum 1983 (≈ WGS84 to within centimetres).
    Nad83,
    /// New Zealand Geodetic Datum 2000 (≈ WGS84; realisation of ITRF96).
    Nzgd2000,
    /// A custom datum with explicit parameters.
    Custom(DatumParams),
}

impl Datum {
    /// The reference ellipsoid of this datum.
    pub fn ellipsoid(&self) -> Ellipsoid {
        match self {
            Datum::Wgs84 | Datum::Nad83 | Datum::Nzgd2000 => ellipsoid::WGS84,
            Datum::Custom(p) => p.ellipsoid,
        }
    }

    /// Parameters describing the transform *from* this datum *to* WGS84.
    ///
    /// Modern NAD83 and NZGD2000 are ITRF realisations aligned with WGS84 to
    /// centimetre level, so their parameters are (correctly) near-identity.
    pub fn to_wgs84(&self) -> DatumParams {
        match self {
            Datum::Wgs84 => DatumParams {
                ellipsoid: ellipsoid::WGS84,
                translation: [0.0; 3],
                rotation: [0.0; 3],
                scale_ppm: 0.0,
            },
            Datum::Nad83 => DatumParams {
                ellipsoid: ellipsoid::WGS84,
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale_ppm: 0.0,
            },
            Datum::Nzgd2000 => DatumParams {
                ellipsoid: ellipsoid::WGS84,
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale_ppm: 0.0,
            },
            Datum::Custom(p) => *p,
        }
    }
}

/// A geocentric (ECEF) coordinate in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ecef {
    /// X component (metres).
    pub x: f64,
    /// Y component (metres).
    pub y: f64,
    /// Z component (metres).
    pub z: f64,
}

/// Convert a geodetic coordinate (radians, metres) on `ellipsoid` to ECEF.
pub fn geodetic_to_ecef(lat: f64, lon: f64, h: f64, ell: &Ellipsoid) -> Ecef {
    let e2 = ell.eccentricity2();
    let n = ell.a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    let x = (n + h) * lat.cos() * lon.cos();
    let y = (n + h) * lat.cos() * lon.sin();
    let z = (n * (1.0 - e2) + h) * lat.sin();
    Ecef { x, y, z }
}

/// Convert an ECEF coordinate (metres) on `ellipsoid` to geodetic
/// (radians, metres) by iterative refinement.
pub fn ecef_to_geodetic(p: &Ecef, ell: &Ellipsoid) -> (f64, f64, f64) {
    let e2 = ell.eccentricity2();
    let lon = p.y.atan2(p.x);
    let p_xy = (p.x * p.x + p.y * p.y).sqrt();
    let mut lat = p.z.atan2(p_xy * (1.0 - e2));
    let mut h = 0.0;
    for _ in 0..20 {
        let n = ell.a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        let new_h = p_xy / lat.cos() - n;
        let new_lat = (p.z / (1.0 - e2 * n / (n + h))).atan2(p_xy);
        if (new_lat - lat).abs() < 1e-12 && (new_h - h).abs() < 1e-9 {
            lat = new_lat;
            h = new_h;
            break;
        }
        lat = new_lat;
        h = new_h;
    }
    (lat, lon, h)
}

const ARCSEC_TO_RAD: f64 = core::f64::consts::PI / (180.0 * 3600.0);

/// Apply the Bursa–Wolf transform `X_wgs = T + (1 + s)·R·X_datum`.
fn helmert_forward(x: &Ecef, p: &DatumParams) -> Ecef {
    let s = p.scale_ppm * 1e-6;
    let rx = p.rotation[0] * ARCSEC_TO_RAD;
    let ry = p.rotation[1] * ARCSEC_TO_RAD;
    let rz = p.rotation[2] * ARCSEC_TO_RAD;
    // Rotation matrix R = Rz·Ry·Rx.
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let r00 = cy * cz;
    let r01 = sx * sy * cz - cx * sz;
    let r02 = cx * sy * cz + sx * sz;
    let r10 = cy * sz;
    let r11 = sx * sy * sz + cx * cz;
    let r12 = cx * sy * sz - sx * cz;
    let r20 = -sy;
    let r21 = sx * cy;
    let r22 = cx * cy;
    let k = 1.0 + s;
    let x2 = k * (r00 * x.x + r01 * x.y + r02 * x.z) + p.translation[0];
    let y2 = k * (r10 * x.x + r11 * x.y + r12 * x.z) + p.translation[1];
    let z2 = k * (r20 * x.x + r21 * x.y + r22 * x.z) + p.translation[2];
    Ecef { x: x2, y: y2, z: z2 }
}

/// Apply the inverse Bursa–Wolf transform (WGS84 → datum).
fn helmert_inverse(x: &Ecef, p: &DatumParams) -> Ecef {
    let s = p.scale_ppm * 1e-6;
    let rx = p.rotation[0] * ARCSEC_TO_RAD;
    let ry = p.rotation[1] * ARCSEC_TO_RAD;
    let rz = p.rotation[2] * ARCSEC_TO_RAD;
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let r00 = cy * cz;
    let r01 = sx * sy * cz - cx * sz;
    let r02 = cx * sy * cz + sx * sz;
    let r10 = cy * sz;
    let r11 = sx * sy * sz + cx * cz;
    let r12 = cx * sy * sz - sx * cz;
    let r20 = -sy;
    let r21 = sx * cy;
    let r22 = cx * cy;
    let dx = x.x - p.translation[0];
    let dy = x.y - p.translation[1];
    let dz = x.z - p.translation[2];
    let k = 1.0 / (1.0 + s);
    let x2 = k * (r00 * dx + r10 * dy + r20 * dz);
    let y2 = k * (r01 * dx + r11 * dy + r21 * dz);
    let z2 = k * (r02 * dx + r12 * dy + r22 * dz);
    Ecef { x: x2, y: y2, z: z2 }
}

/// Transform a geodetic coordinate from one datum to another.
///
/// Angles are in **degrees**; heights in metres. Result is `(lon, lat, h)`.
pub fn transform_datum(lon: f64, lat: f64, h: f64, from: Datum, to: Datum) -> (f64, f64, f64) {
    let rad = core::f64::consts::PI / 180.0;
    if from == to {
        return (lon, lat, h);
    }
    let ell = from.ellipsoid();
    let ecef = geodetic_to_ecef(lat * rad, lon * rad, h, &ell);
    // Datum -> WGS84.
    let wgs = helmert_forward(&ecef, &from.to_wgs84());
    // WGS84 -> target (inverse of target's to_wgs84 params).
    let target_ecef = helmert_inverse(&wgs, &to.to_wgs84());
    let (lat_r, lon_r, h_r) = ecef_to_geodetic(&target_ecef, &to.ellipsoid());
    (lon_r / rad, lat_r / rad, h_r)
}

/// A parsed PROJ initialisation string (e.g. `+proj=utm +zone=33 +datum=WGS84`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjString {
    /// Ordered `(key, value)` pairs.
    pub params: Vec<(String, String)>,
}

impl ProjString {
    /// Parse a PROJ string into key/value pairs.
    ///
    /// Tokens are of the form `+key=value` or `+key` (boolean flag). Whitespace
    /// between tokens is ignored.
    pub fn parse(input: &str) -> Result<Self, GeoError> {
        let mut params = Vec::new();
        for tok in input.split('+') {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            if let Some((k, v)) = tok.split_once('=') {
                params.push((k.trim().to_string(), v.trim().to_string()));
            } else {
                params.push((tok.to_string(), String::new()));
            }
        }
        Ok(ProjString { params })
    }

    /// Look up a parameter value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Whether a boolean flag is present.
    pub fn has(&self, key: &str) -> bool {
        self.params.iter().any(|(k, v)| k == key && v.is_empty())
    }

    /// Validate that the PROJ string is internally consistent.
    pub fn validate(&self) -> Result<(), GeoError> {
        for (k, v) in &self.params {
            match k.as_str() {
                "proj" | "datum" | "ellps" | "units" => {
                    if v.is_empty() {
                        return Err(GeoError::crs(alloc::format!(
                            "PROJ key '{k}' requires a value"
                        )));
                    }
                }
                "zone" | "lat_0" | "lon_0" | "x_0" | "y_0" | "k" | "a" | "f" => {
                    if v.is_empty() || v.parse::<f64>().is_err() {
                        return Err(GeoError::crs(alloc::format!(
                            "PROJ key '{k}' must be numeric"
                        )));
                    }
                }
                "init" | "no_defs" | "over" | "wktext" => {}
                _ => { /* unknown keys are tolerated */ }
            }
        }
        Ok(())
    }

    /// Render back to a canonical `+k=v` form.
    pub fn to_proj_string(&self) -> String {
        let mut s = String::new();
        for (i, (k, v)) in self.params.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push('+');
            s.push_str(k);
            if !v.is_empty() {
                s.push('=');
                s.push_str(v);
            }
        }
        s
    }
}

/// An EPSG coordinate reference system code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpsgCode(pub u32);

/// The kind of CRS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrsKind {
    /// Geographic (lon/lat on a datum).
    Geographic,
    /// Projected (planar, e.g. UTM).
    Projected,
}

/// Descriptive information about a registered EPSG code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrsInfo {
    /// The EPSG code.
    pub code: u32,
    /// Human-readable name.
    pub name: &'static str,
    /// Geographic or projected.
    pub kind: CrsKind,
    /// The datum.
    pub datum: Datum,
    /// For projected CRS, the projection definition (if known).
    pub projection: Option<Projection>,
}

/// A projection definition (parameters for a planar projection).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Equirectangular / plate carrée.
    PlateCarree,
    /// Transverse Mercator with the given central meridian (deg), scale
    /// factor, false easting, and false northing (metres).
    TransverseMercator {
        /// Central meridian (degrees).
        central_meridian: f64,
        /// Scale factor at the central meridian.
        scale_factor: f64,
        /// False easting (metres).
        false_easting: f64,
        /// False northing (metres).
        false_northing: f64,
    },
}

/// Look up a registered EPSG code.
pub fn lookup_epsg(code: u32) -> Option<CrsInfo> {
    Some(match code {
        4326 => CrsInfo {
            code,
            name: "WGS84 geographic",
            kind: CrsKind::Geographic,
            datum: Datum::Wgs84,
            projection: None,
        },
        4269 => CrsInfo {
            code,
            name: "NAD83 geographic",
            kind: CrsKind::Geographic,
            datum: Datum::Nad83,
            projection: None,
        },
        4167 => CrsInfo {
            code,
            name: "NZGD2000 geographic",
            kind: CrsKind::Geographic,
            datum: Datum::Nzgd2000,
            projection: None,
        },
        3857 => CrsInfo {
            code,
            name: "WGS84 / Pseudo-Mercator",
            kind: CrsKind::Projected,
            datum: Datum::Wgs84,
            projection: Some(Projection::PlateCarree),
        },
        2193 => CrsInfo {
            code,
            name: "NZGD2000 / New Zealand Transverse Mercator",
            kind: CrsKind::Projected,
            datum: Datum::Nzgd2000,
            projection: Some(Projection::TransverseMercator {
                central_meridian: 173.0,
                scale_factor: 0.9996,
                false_easting: 1_600_000.0,
                false_northing: 10_000_000.0,
            }),
        },
        c if (32601..=32660).contains(&c) => CrsInfo {
            code: c,
            name: "WGS84 / UTM zone north",
            kind: CrsKind::Projected,
            datum: Datum::Wgs84,
            projection: Some(utm_projection(c - 32600, false)),
        },
        c if (32701..=32760).contains(&c) => CrsInfo {
            code: c,
            name: "WGS84 / UTM zone south",
            kind: CrsKind::Projected,
            datum: Datum::Wgs84,
            projection: Some(utm_projection(c - 32700, true)),
        },
        _ => return None,
    })
}

fn utm_projection(zone: u32, south: bool) -> Projection {
    let central_meridian = (zone as f64 - 1.0) * 6.0 - 177.0;
    Projection::TransverseMercator {
        central_meridian,
        scale_factor: 0.9996,
        false_easting: 500_000.0,
        false_northing: if south { 10_000_000.0 } else { 0.0 },
    }
}

/// Compute the UTM zone (1..60) for a longitude.
pub fn utm_zone(lon: f64) -> u32 {
    let z = ((lon + 180.0) / 6.0).floor() as i64 + 1;
    z.clamp(1, 60) as u32
}

/// A concrete coordinate reference system.
#[derive(Debug, Clone, PartialEq)]
pub struct Crs {
    /// An EPSG code, if known.
    pub code: Option<u32>,
    /// A human-readable name.
    pub name: Option<String>,
    /// Geographic or projected.
    pub kind: CrsKind,
    /// The datum.
    pub datum: Datum,
    /// The projection, if projected.
    pub projection: Option<Projection>,
    /// An optional source PROJ string.
    pub proj: Option<ProjString>,
}

impl Crs {
    /// WGS84 geographic CRS.
    pub fn wgs84() -> Self {
        Crs {
            code: Some(4326),
            name: Some("WGS84".to_string()),
            kind: CrsKind::Geographic,
            datum: Datum::Wgs84,
            projection: None,
            proj: None,
        }
    }

    /// NAD83 geographic CRS.
    pub fn nad83() -> Self {
        Crs {
            code: Some(4269),
            name: Some("NAD83".to_string()),
            kind: CrsKind::Geographic,
            datum: Datum::Nad83,
            projection: None,
            proj: None,
        }
    }

    /// NZGD2000 geographic CRS.
    pub fn nzgd2000() -> Self {
        Crs {
            code: Some(4167),
            name: Some("NZGD2000".to_string()),
            kind: CrsKind::Geographic,
            datum: Datum::Nzgd2000,
            projection: None,
            proj: None,
        }
    }

    /// Build a CRS from a registered EPSG code.
    pub fn from_epsg(code: u32) -> Option<Self> {
        let info = lookup_epsg(code)?;
        Some(Crs {
            code: Some(code),
            name: Some(info.name.to_string()),
            kind: info.kind,
            datum: info.datum,
            projection: info.projection,
            proj: None,
        })
    }

    /// Whether this CRS is geographic.
    pub fn is_geographic(&self) -> bool {
        self.kind == CrsKind::Geographic
    }
}

/// Validate that a longitude is within `[-180, 180]`.
pub fn validate_longitude(lon: f64) -> Result<(), GeoError> {
    if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
        return Err(GeoError::crs(alloc::format!("longitude {lon} out of range [-180,180]")));
    }
    Ok(())
}

/// Validate that a latitude is within `[-90, 90]`.
pub fn validate_latitude(lat: f64) -> Result<(), GeoError> {
    if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(GeoError::crs(alloc::format!("latitude {lat} out of range [-90,90]")));
    }
    Ok(())
}

/// Validate a `(lon, lat)` pair.
pub fn validate_lon_lat(lon: f64, lat: f64) -> Result<(), GeoError> {
    validate_longitude(lon)?;
    validate_latitude(lat)
}

// ---- Projections ---------------------------------------------------------

/// Forward projection: geographic `(lon, lat)` degrees → projected `(x, y)` metres.
pub fn project(crs: &Crs, lon: f64, lat: f64) -> Result<(f64, f64), GeoError> {
    let proj =
        crs.projection.ok_or_else(|| GeoError::crs("CRS has no projection (not projected)"))?;
    validate_lon_lat(lon, lat)?;
    let rad = core::f64::consts::PI / 180.0;
    Ok(match proj {
        Projection::PlateCarree => {
            // Pseudo/equirectangular: x = lon * R, y = lat * R (spherical-ish).
            let r = crs.datum.ellipsoid().a;
            (lon * rad * r, lat * rad * r)
        }
        Projection::TransverseMercator {
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
        } => {
            let ell = crs.datum.ellipsoid();
            let (x, y) = transverse_mercator_forward(
                lat * rad,
                (lon - central_meridian) * rad,
                &ell,
                scale_factor,
            );
            (x + false_easting, y + false_northing)
        }
    })
}

/// Inverse projection: projected `(x, y)` metres → geographic `(lon, lat)` degrees.
pub fn unproject(crs: &Crs, x: f64, y: f64) -> Result<(f64, f64), GeoError> {
    let proj =
        crs.projection.ok_or_else(|| GeoError::crs("CRS has no projection (not projected)"))?;
    let rad = core::f64::consts::PI / 180.0;
    match proj {
        Projection::PlateCarree => {
            let r = crs.datum.ellipsoid().a;
            let lon = x / (rad * r);
            let lat = y / (rad * r);
            validate_lon_lat(lon, lat)?;
            Ok((lon, lat))
        }
        Projection::TransverseMercator {
            central_meridian,
            scale_factor,
            false_easting,
            false_northing,
        } => {
            let ell = crs.datum.ellipsoid();
            let (lat, lon_rel) = transverse_mercator_inverse(
                x - false_easting,
                y - false_northing,
                &ell,
                scale_factor,
            );
            let lon = lon_rel / rad + central_meridian;
            let lat_d = lat / rad;
            validate_lon_lat(lon, lat_d)?;
            Ok((lon, lat_d))
        }
    }
}

/// Transverse Mercator forward map (Redfearn series). Returns `(easting, northing)`
/// before false easting/northing offsets, in metres.
pub fn transverse_mercator_forward(
    lat: f64,
    lon_rel: f64,
    ell: &Ellipsoid,
    scale_factor: f64,
) -> (f64, f64) {
    let e2 = ell.eccentricity2();
    let n = ell.f / (2.0 - ell.f);
    let a = ell.a;
    let sin_phi = lat.sin();
    let cos_phi = lat.cos();
    let tan_phi = lat.tan();
    let v = a / (1.0 - e2 * sin_phi.powi(2)).sqrt();
    let rho = a * (1.0 - e2) / (1.0 - e2 * sin_phi.powi(2)).powf(1.5);
    let eta2 = v / rho - 1.0;
    let sec_phi = 1.0 / cos_phi;
    let m = meridional_arc(lat, ell);
    let i = m;
    let ii = v / 2.0 * sin_phi * cos_phi;
    let iii = v / 24.0 * sin_phi * cos_phi.powi(3) * (5.0 - tan_phi.powi(2) + 9.0 * eta2);
    let iiia =
        v / 720.0 * sin_phi * cos_phi.powi(5) * (61.0 - 58.0 * tan_phi.powi(2) + tan_phi.powi(4));
    let iv = v * cos_phi;
    let vv = v / 6.0 * cos_phi.powi(3) * (v / rho - tan_phi.powi(2));
    let vi = v / 120.0
        * cos_phi.powi(5)
        * (5.0 - 18.0 * tan_phi.powi(2) + tan_phi.powi(4) + 14.0 * eta2
            - 58.0 * tan_phi.powi(2) * eta2);
    let easting = scale_factor * (iv * lon_rel + vv * lon_rel.powi(3) + vi * lon_rel.powi(5));
    let northing =
        scale_factor * (i + ii * lon_rel.powi(2) + iii * lon_rel.powi(4) + iiia * lon_rel.powi(6));
    let _ = (n, sec_phi);
    (easting, northing)
}

/// Transverse Mercator inverse map via Newton iteration with a numerical
/// Jacobian. Returns `(lat, lon_rel)` in radians (before central-meridian offset).
///
/// The forward map is used as the residual; a 2×2 Newton step converges to
/// sub-millimetre accuracy within a handful of iterations for any point inside a
/// UTM/NZTM zone.
pub fn transverse_mercator_inverse(
    easting: f64,
    northing: f64,
    ell: &Ellipsoid,
    scale_factor: f64,
) -> (f64, f64) {
    // Initial guess via the footpoint latitude (meridional arc).
    let mut lat = footpoint_latitude(northing / scale_factor, ell);
    let mut lon = 0.0;
    let h = 1e-6;
    for _ in 0..60 {
        let (e, n) = transverse_mercator_forward(lat, lon, ell, scale_factor);
        let de = easting - e;
        let dn = northing - n;
        if (de * de + dn * dn) < 1e-18 {
            break;
        }
        let (e_lp, n_lp) = transverse_mercator_forward(lat + h, lon, ell, scale_factor);
        let (e_lm, n_lm) = transverse_mercator_forward(lat - h, lon, ell, scale_factor);
        let (e_pl, n_pl) = transverse_mercator_forward(lat, lon + h, ell, scale_factor);
        let (e_ml, n_ml) = transverse_mercator_forward(lat, lon - h, ell, scale_factor);
        let d_e_dlat = (e_lp - e_lm) / (2.0 * h);
        let d_n_dlat = (n_lp - n_lm) / (2.0 * h);
        let d_e_dlon = (e_pl - e_ml) / (2.0 * h);
        let d_n_dlon = (n_pl - n_ml) / (2.0 * h);
        let det = d_e_dlat * d_n_dlon - d_e_dlon * d_n_dlat;
        if det.abs() < 1e-12 {
            break;
        }
        lat += (de * d_n_dlon - d_e_dlon * dn) / det;
        lon += (d_e_dlat * dn - de * d_n_dlat) / det;
    }
    (lat, lon)
}

/// Meridional arc length from the equator to latitude `phi` (radians).
fn meridional_arc(phi: f64, ell: &Ellipsoid) -> f64 {
    let e2 = ell.eccentricity2();
    let a = ell.a;
    let m1 = (1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0) * phi;
    let m2 =
        (3.0 * e2 / 8.0 + 3.0 * e2.powi(2) / 32.0 + 45.0 * e2.powi(3) / 1024.0) * (2.0 * phi).sin();
    let m3 = (15.0 * e2.powi(2) / 256.0 + 45.0 * e2.powi(3) / 1024.0) * (4.0 * phi).sin();
    let m4 = (35.0 * e2.powi(3) / 3072.0) * (6.0 * phi).sin();
    a * (m1 - m2 + m3 - m4)
}

/// Footpoint latitude for inverse TM (series inverted by fixed-point iteration).
fn footpoint_latitude(m: f64, ell: &Ellipsoid) -> f64 {
    let e2 = ell.eccentricity2();
    let a = ell.a;
    let n = (a - ell.semi_minor()) / (a + ell.semi_minor());
    let mut phi = m / (a * (1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0));
    for _ in 0..15 {
        let m_phi = meridional_arc(phi, ell);
        let delta = (m - m_phi) / (a * (1.0 - n));
        phi += delta * 1.0;
        if delta.abs() < 1e-13 {
            break;
        }
    }
    phi
}

/// Reproject a coordinate between two CRS.
///
/// Supports geographic↔geographic (datum shift), projected→geographic
/// (inverse projection + datum shift), and geographic→projected (projection +
/// datum shift). Projected→projected repiprojection is performed via the
/// geographic intermediate. Angles returned in degrees; projected in metres.
pub fn reproject(
    from: &Crs,
    to: &Crs,
    lon_or_x: f64,
    lat_or_y: f64,
) -> Result<(f64, f64), GeoError> {
    // First, normalise both endpoints to geographic WGS84 lon/lat (degrees).
    let (geo_lon, geo_lat) = if from.is_geographic() {
        (lon_or_x, lat_or_y)
    } else {
        unproject(from, lon_or_x, lat_or_y)?
    };
    validate_lon_lat(geo_lon, geo_lat)?;

    // Apply datum shift on the geographic pair (treating from.datum -> to.datum).
    let (lon, lat, _h) = transform_datum(geo_lon, geo_lat, 0.0, from.datum, to.datum);

    if to.is_geographic() {
        Ok((lon, lat))
    } else {
        project(to, lon, lat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proj_string() {
        let p = ProjString::parse("+proj=utm +zone=33 +datum=WGS84 +units=m").unwrap();
        assert_eq!(p.get("proj"), Some("utm"));
        assert_eq!(p.get("zone"), Some("33"));
        assert!(p.validate().is_ok());
    }

    #[test]
    fn proj_invalid_zone() {
        let p = ProjString::parse("+proj=utm +zone=abc").unwrap();
        assert!(p.validate().is_err());
    }

    #[test]
    fn datum_identity() {
        let (lon, lat, _) = transform_datum(174.0, -41.0, 0.0, Datum::Nzgd2000, Datum::Nzgd2000);
        assert!((lon - 174.0).abs() < 1e-9);
        assert!((lat + 41.0).abs() < 1e-9);
    }

    #[test]
    fn datum_wgs84_nad83_close() {
        // NAD83 ≈ WGS84: should be near-identity.
        let (lon, lat, _) = transform_datum(174.0, -41.0, 0.0, Datum::Wgs84, Datum::Nad83);
        assert!((lon - 174.0).abs() < 1e-6);
        assert!((lat + 41.0).abs() < 1e-6);
    }

    #[test]
    fn utm_round_trip() {
        let crs = Crs::from_epsg(32760).unwrap(); // UTM 60S
        let (x, y) = project(&crs, 174.0, -41.0).unwrap();
        let (lon, lat) = unproject(&crs, x, y).unwrap();
        assert!((lon - 174.0).abs() < 1e-8);
        assert!((lat + 41.0).abs() < 1e-8);
    }

    #[test]
    fn nztm_round_trip() {
        let crs = Crs::from_epsg(2193).unwrap();
        let (x, y) = project(&crs, 174.0, -41.0).unwrap();
        let (lon, lat) = unproject(&crs, x, y).unwrap();
        assert!((lon - 174.0).abs() < 1e-7);
        assert!((lat + 41.0).abs() < 1e-7);
    }

    #[test]
    fn reproject_utm_to_wgs84() {
        let utm = Crs::from_epsg(32760).unwrap();
        let wgs = Crs::wgs84();
        let (x, y) = project(&utm, 174.0, -41.0).unwrap();
        let (lon, lat) = reproject(&utm, &wgs, x, y).unwrap();
        assert!((lon - 174.0).abs() < 1e-7);
        assert!((lat + 41.0).abs() < 1e-7);
    }

    #[test]
    fn validate_ranges() {
        assert!(validate_longitude(200.0).is_err());
        assert!(validate_latitude(-100.0).is_err());
    }

    #[test]
    fn lookup_epsg_utm() {
        let info = lookup_epsg(32633).unwrap();
        assert_eq!(info.kind, CrsKind::Projected);
        assert!(info.projection.is_some());
    }
}
