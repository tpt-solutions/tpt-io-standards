//! Orbit Ephemeris Message (OEM): segments of Cartesian state vectors.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::CcsdsError;
use crate::kvn::{classify, lines, number, Line};

/// The metadata block of an OEM segment (the keys this crate uses; others are kept in `extra`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OemMeta {
    /// `OBJECT_NAME`.
    pub object_name: String,
    /// `OBJECT_ID`, usually the international designator.
    pub object_id: String,
    /// `CENTER_NAME`, e.g. `EARTH`.
    pub center_name: String,
    /// `REF_FRAME`, e.g. `EME2000`.
    pub ref_frame: String,
    /// `TIME_SYSTEM`, e.g. `UTC`.
    pub time_system: String,
    /// `START_TIME`, as written.
    pub start_time: String,
    /// `STOP_TIME`, as written.
    pub stop_time: String,
    /// Other metadata keys in file order.
    pub extra: Vec<(String, String)>,
}

/// One state: epoch text, position in kilometres, velocity in kilometres per second.
#[derive(Debug, Clone, PartialEq)]
pub struct StateVector {
    /// Epoch as written in the file.
    pub epoch: String,
    /// Position `[x, y, z]`, km.
    pub position: [f64; 3],
    /// Velocity `[x, y, z]`, km/s.
    pub velocity: [f64; 3],
}

/// One ephemeris segment: metadata and its states.
#[derive(Debug, Clone, PartialEq)]
pub struct OemSegment {
    /// The segment's metadata.
    pub meta: OemMeta,
    /// State vectors in file order.
    pub states: Vec<StateVector>,
}

/// A whole OEM file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Oem {
    /// `CCSDS_OEM_VERS`, e.g. `2.0`.
    pub version: String,
    /// `CREATION_DATE`, if present.
    pub creation_date: Option<String>,
    /// `ORIGINATOR`, if present.
    pub originator: Option<String>,
    /// Segments in file order.
    pub segments: Vec<OemSegment>,
}

/// Parse an OEM file.
pub fn parse(input: &[u8]) -> Result<Oem, CcsdsError> {
    let mut oem = Oem::default();
    let mut current: Option<OemSegment> = None;
    let mut in_meta = false;
    let mut in_cov = false;

    for raw in lines(input)? {
        let Some(line) = classify(raw) else { continue };
        if in_cov {
            if matches!(line, Line::Keyword("COVARIANCE_STOP")) {
                in_cov = false;
            }
            continue;
        }
        match line {
            Line::Pair("CCSDS_OEM_VERS", v) => oem.version = v.to_string(),
            Line::Pair("CREATION_DATE", v) => oem.creation_date = Some(v.to_string()),
            Line::Pair("ORIGINATOR", v) => oem.originator = Some(v.to_string()),
            Line::Keyword("META_START") => {
                if let Some(seg) = current.take() {
                    oem.segments.push(seg);
                }
                in_meta = true;
                current = Some(OemSegment { meta: OemMeta::default(), states: Vec::new() });
            }
            Line::Keyword("META_STOP") => in_meta = false,
            Line::Keyword("COVARIANCE_START") => in_cov = true,
            Line::Pair(k, v) if in_meta => {
                let seg = current
                    .as_mut()
                    .ok_or_else(|| CcsdsError::Invalid("metadata outside a segment".into()))?;
                set_meta(&mut seg.meta, k, v);
            }
            Line::Other(text) => {
                let seg = current
                    .as_mut()
                    .ok_or_else(|| CcsdsError::Invalid("state vector outside a segment".into()))?;
                seg.states.push(state(text)?);
            }
            _ => {}
        }
    }
    if let Some(seg) = current.take() {
        oem.segments.push(seg);
    }
    if oem.version.is_empty() {
        return Err(CcsdsError::Invalid("missing CCSDS_OEM_VERS".into()));
    }
    Ok(oem)
}

fn set_meta(m: &mut OemMeta, key: &str, value: &str) {
    match key {
        "OBJECT_NAME" => m.object_name = value.to_string(),
        "OBJECT_ID" => m.object_id = value.to_string(),
        "CENTER_NAME" => m.center_name = value.to_string(),
        "REF_FRAME" => m.ref_frame = value.to_string(),
        "TIME_SYSTEM" => m.time_system = value.to_string(),
        "START_TIME" => m.start_time = value.to_string(),
        "STOP_TIME" => m.stop_time = value.to_string(),
        _ => m.extra.push((key.to_string(), value.to_string())),
    }
}

/// `epoch x y z vx vy vz`; an optional acceleration triple is ignored.
fn state(text: &str) -> Result<StateVector, CcsdsError> {
    let cols: Vec<&str> = text.split_whitespace().collect();
    if cols.len() < 7 {
        return Err(CcsdsError::Invalid("state vector needs an epoch and six numbers".into()));
    }
    Ok(StateVector {
        epoch: cols[0].to_string(),
        position: [number(cols[1], "x")?, number(cols[2], "y")?, number(cols[3], "z")?],
        velocity: [number(cols[4], "vx")?, number(cols[5], "vy")?, number(cols[6], "vz")?],
    })
}

/// Serialise an OEM. Numbers are written with Rust's shortest round-trip formatting, so
/// `parse(write(x)) == x`.
pub fn write(oem: &Oem) -> Vec<u8> {
    let mut out = alloc::string::String::new();
    let _ = writeln!(out, "CCSDS_OEM_VERS = {}", oem.version);
    if let Some(d) = &oem.creation_date {
        let _ = writeln!(out, "CREATION_DATE = {d}");
    }
    if let Some(o) = &oem.originator {
        let _ = writeln!(out, "ORIGINATOR = {o}");
    }
    for seg in &oem.segments {
        out.push('\n');
        out.push_str("META_START\n");
        let m = &seg.meta;
        let _ = writeln!(out, "OBJECT_NAME = {}", m.object_name);
        let _ = writeln!(out, "OBJECT_ID = {}", m.object_id);
        let _ = writeln!(out, "CENTER_NAME = {}", m.center_name);
        let _ = writeln!(out, "REF_FRAME = {}", m.ref_frame);
        let _ = writeln!(out, "TIME_SYSTEM = {}", m.time_system);
        let _ = writeln!(out, "START_TIME = {}", m.start_time);
        let _ = writeln!(out, "STOP_TIME = {}", m.stop_time);
        for (k, v) in &m.extra {
            let _ = writeln!(out, "{k} = {v}");
        }
        out.push_str("META_STOP\n");
        for s in &seg.states {
            let p = s.position;
            let v = s.velocity;
            let _ =
                writeln!(out, "{} {} {} {} {} {} {}", s.epoch, p[0], p[1], p[2], v[0], v[1], v[2]);
        }
    }
    out.into_bytes()
}
