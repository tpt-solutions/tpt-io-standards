//! OME-Zarr metadata: `.zattrs` multiscales and `.zarray` array descriptions.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{parse_json, Json};

use crate::error::OmeError;

/// One multiscale image: its name, axes and dataset paths (one per resolution level).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Multiscales {
    /// The multiscale name, if present.
    pub name: Option<String>,
    /// The `(name, type)` axes, e.g. `("z", "space")`.
    pub axes: Vec<(String, String)>,
    /// The dataset paths, in resolution order (full resolution first).
    pub datasets: Vec<String>,
}

/// A Zarr array description from `.zarray`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ZArray {
    /// Array shape.
    pub shape: Vec<u64>,
    /// Chunk shape.
    pub chunks: Vec<u64>,
    /// The NumPy-style dtype, e.g. `<u2`.
    pub dtype: String,
    /// `C` (row-major) or `F` (column-major) order.
    pub order: String,
    /// `true` when no compressor is set (chunks are stored raw).
    pub uncompressed: bool,
}

fn text(v: Option<&Json>) -> Option<String> {
    v.and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn numbers(v: Option<&Json>) -> Result<Vec<u64>, OmeError> {
    let arr = v
        .and_then(|x| x.as_array())
        .ok_or_else(|| OmeError::Invalid("expected an array of sizes".to_string()))?;
    arr.iter()
        .map(|n| {
            n.as_f64()
                .filter(|x| *x >= 0.0 && x.fract() == 0.0)
                .map(|x| x as u64)
                .ok_or_else(|| OmeError::Invalid("size is not a non-negative integer".to_string()))
        })
        .collect()
}

/// Parse a `.zattrs` document and return its first multiscale entry.
pub fn parse_zattrs(input: &[u8]) -> Result<Multiscales, OmeError> {
    let root = parse_json(input)?;
    let ms = root
        .get("multiscales")
        .and_then(|m| m.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| OmeError::Invalid("no multiscales entry".to_string()))?;
    let axes = ms
        .get("axes")
        .and_then(|a| a.as_array())
        .unwrap_or(&[])
        .iter()
        .map(|a| (text(a.get("name")).unwrap_or_default(), text(a.get("type")).unwrap_or_default()))
        .collect();
    let datasets = ms
        .get("datasets")
        .and_then(|d| d.as_array())
        .unwrap_or(&[])
        .iter()
        .filter_map(|d| text(d.get("path")))
        .collect();
    Ok(Multiscales { name: text(ms.get("name")), axes, datasets })
}

/// Parse a `.zarray` document.
pub fn parse_zarray(input: &[u8]) -> Result<ZArray, OmeError> {
    let root = parse_json(input)?;
    if root.get("zarr_format").and_then(|v| v.as_f64()) != Some(2.0) {
        return Err(OmeError::Invalid("only Zarr format 2 is read".to_string()));
    }
    let uncompressed = matches!(root.get("compressor"), None | Some(Json::Null));
    Ok(ZArray {
        shape: numbers(root.get("shape"))?,
        chunks: numbers(root.get("chunks"))?,
        dtype: text(root.get("dtype")).unwrap_or_default(),
        order: text(root.get("order")).unwrap_or_else(|| "C".to_string()),
        uncompressed,
    })
}
