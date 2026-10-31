//! Tracking Data Message (TDM): header, metadata and data blocks as ordered key/value lists.
//! Read only; individual observations are not grouped.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::CcsdsError;
use crate::kvn::{classify, lines, Line};

/// The key/value pairs of one `META_START … META_STOP` or `DATA_START … DATA_STOP` block.
pub type TdmBlock = Vec<(String, String)>;

/// A parsed TDM file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tdm {
    /// `CCSDS_TDM_VERS`, e.g. `2.0`.
    pub version: String,
    /// Pairs before the first segment (the header).
    pub header: TdmBlock,
    /// Each segment as its metadata block and its data block.
    pub segments: Vec<(TdmBlock, TdmBlock)>,
}

/// Parse a TDM file.
pub fn parse(input: &[u8]) -> Result<Tdm, CcsdsError> {
    let mut tdm = Tdm::default();
    let mut meta: Option<TdmBlock> = None;
    let mut data: Option<TdmBlock> = None;
    let mut seen_segment = false;

    for raw in lines(input)? {
        match classify(raw) {
            Some(Line::Keyword("META_START")) => {
                meta = Some(Vec::new());
                seen_segment = true;
            }
            Some(Line::Keyword("META_STOP")) => {}
            Some(Line::Keyword("DATA_START")) => data = Some(Vec::new()),
            Some(Line::Keyword("DATA_STOP")) => {
                let d = data.take().ok_or_else(|| CcsdsError::Invalid("DATA_STOP without DATA_START".into()))?;
                let m = meta.take().unwrap_or_default();
                tdm.segments.push((m, d));
            }
            Some(Line::Pair(k, v)) => {
                if k == "CCSDS_TDM_VERS" {
                    tdm.version = v.to_string();
                }
                let pair = (k.to_string(), v.to_string());
                if let Some(d) = data.as_mut() {
                    d.push(pair);
                } else if let Some(m) = meta.as_mut() {
                    m.push(pair);
                } else if !seen_segment {
                    tdm.header.push(pair);
                }
            }
            _ => {}
        }
    }
    if tdm.version.is_empty() {
        return Err(CcsdsError::Invalid("missing CCSDS_TDM_VERS".into()));
    }
    Ok(tdm)
}
