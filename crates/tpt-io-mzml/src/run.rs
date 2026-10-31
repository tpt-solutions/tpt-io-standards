//! Spectrum extraction from an mzML document.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::base64::decode;
use crate::error::MzmlError;

/// One spectrum: its id, MS level, retention time (seconds) and its m/z and intensity arrays.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spectrum {
    /// The spectrum's `index` attribute.
    pub index: usize,
    /// The spectrum's `id`, e.g. `scan=1`.
    pub id: String,
    /// MS level (`MS:1000511`), if given.
    pub ms_level: Option<u32>,
    /// Scan start time in seconds (`MS:1000016`), if given.
    pub retention_time: Option<f64>,
    /// The m/z array.
    pub mz: Vec<f64>,
    /// The intensity array.
    pub intensity: Vec<f64>,
}

/// An mzML run: its id and spectra in document order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Run {
    /// The run's `id` attribute, if present.
    pub id: Option<String>,
    /// Spectra in document order.
    pub spectra: Vec<Spectrum>,
}

/// Parse an mzML document and decode its spectra.
pub fn parse(input: &[u8]) -> Result<Run, MzmlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "mzML" {
        return Err(ParseError::custom(root.offset, "root element is not <mzML>").into());
    }
    let run = root
        .child("run")
        .ok_or_else(|| MzmlError::Invalid("no <run> element".into()))?;
    let mut out = Run { id: run.attr("id").map(|s| s.to_string()), spectra: Vec::new() };
    if let Some(list) = run.child("spectrumList") {
        for s in list.children_named("spectrum") {
            out.spectra.push(spectrum(s)?);
        }
    }
    Ok(out)
}

/// The value of the first `cvParam` with the given accession.
fn cv<'a>(el: &'a Element, accession: &str) -> Option<&'a Element> {
    el.children_named("cvParam").find(|c| c.attr("accession") == Some(accession))
}

fn spectrum(el: &Element) -> Result<Spectrum, MzmlError> {
    let id = el
        .attr("id")
        .ok_or_else(|| MzmlError::Invalid("spectrum has no id".into()))?
        .to_string();
    let index = el
        .attr("index")
        .and_then(|i| i.parse::<usize>().ok())
        .unwrap_or(0);
    let ms_level = cv(el, "MS:1000511")
        .and_then(|c| c.attr("value"))
        .and_then(|v| v.parse::<u32>().ok());
    // The scan start time is found in the scan list; its unit decides the scale.
    let retention_time = el.descendants_named("cvParam").into_iter().find(|c| c.attr("accession") == Some("MS:1000016")).and_then(|c| {
        let v = c.attr("value")?.parse::<f64>().ok()?;
        match c.attr("unitName") {
            Some("minute") => Some(v * 60.0),
            _ => Some(v),
        }
    });
    let mut out = Spectrum { index, id, ms_level, retention_time, mz: Vec::new(), intensity: Vec::new() };
    if let Some(list) = el.child("binaryDataArrayList") {
        for arr in list.children_named("binaryDataArray") {
            let values = array(arr)?;
            if cv(arr, "MS:1000514").is_some() {
                out.mz = values;
            } else if cv(arr, "MS:1000515").is_some() {
                out.intensity = values;
            }
        }
    }
    Ok(out)
}

/// Decode one binary data array into `f64` values.
fn array(arr: &Element) -> Result<Vec<f64>, MzmlError> {
    if cv(arr, "MS:1000574").is_some() {
        return Err(MzmlError::Unsupported("zlib-compressed arrays".into()));
    }
    let bits64 = cv(arr, "MS:1000523").is_some();
    let bits32 = cv(arr, "MS:1000521").is_some();
    if !bits64 && !bits32 {
        return Err(MzmlError::Unsupported("array precision other than 32 or 64 bits".into()));
    }
    let text = arr
        .child("binary")
        .map(|b| b.text_trimmed())
        .unwrap_or("");
    let bytes = decode(text)?;
    let width = if bits64 { 8 } else { 4 };
    if bytes.len() % width != 0 {
        return Err(ParseError::custom(arr.offset, "array length is not a whole number of values").into());
    }
    Ok(bytes
        .chunks_exact(width)
        .map(|c| {
            if bits64 {
                let mut b = [0u8; 8];
                b.copy_from_slice(c);
                f64::from_le_bytes(b)
            } else {
                f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64
            }
        })
        .collect())
}
