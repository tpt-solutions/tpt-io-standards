//! Streaming parsers for large datasets.
//!
//! Loading a multi-megabyte GeoJSON `FeatureCollection` or Shapefile entirely
//! into memory is wasteful when the consumer only needs to visit features one
//! at a time. These readers expose an iterator-like `next` API that keeps at
//! most one feature / record resident at a time. They operate over a `&[u8]`
//! buffer (e.g. a memory-mapped file), so there is no up-front copy.

use alloc::vec::Vec;

use crate::error::GeoError;
use crate::geojson::{parse_feature_json, Feature};
use crate::json::parse;
use crate::shapefile::{parse_shp_record_at, ShapeRecord};

/// Find the byte index of the closing `}` that matches the `{` at `start`.
///
/// Tracks nested braces, brackets, and string literals so braces inside strings
/// or nested arrays are ignored.
fn find_object_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
    let mut in_string = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'{' | b'[' | b'(' => depth += 1,
                b'}' | b']' | b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// A streaming FeatureCollection reader.
///
/// ```
/// use tpt_io_geo::stream::GeoJsonFeatureStream;
/// let src = br#"{"type":"FeatureCollection","features":[
///   {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":{}},
///   {"type":"Feature","geometry":{"type":"Point","coordinates":[3,4]},"properties":{}}
/// ]}"#;
/// let mut stream = GeoJsonFeatureStream::new(src).unwrap();
/// let mut count = 0;
/// while let Some(Ok(_f)) = stream.next() { count += 1; }
/// assert_eq!(count, 2);
/// ```
pub struct GeoJsonFeatureStream<'a> {
    bytes: &'a [u8],
    pos: usize,
    at_end: bool,
}

impl<'a> GeoJsonFeatureStream<'a> {
    /// Position `self` just inside the `features` array.
    pub fn new(bytes: &'a [u8]) -> Result<Self, GeoError> {
        let hay = bytes;
        let mut idx = 0;
        let mut found = None;
        while idx + 9 <= hay.len() {
            if &hay[idx..idx + 9] == b"\"features" {
                found = Some(idx);
                break;
            }
            idx += 1;
        }
        let start = match found {
            Some(f) => f + 9,
            None => {
                let mut p = 0;
                while p < hay.len() && hay[p] != b'{' {
                    p += 1;
                }
                if hay.get(p) == Some(&b'{') && hay[p..].windows(9).any(|w| w == b"\"Feature") {
                    return Ok(Self { bytes, pos: p, at_end: false });
                }
                return Err(GeoError::geojson(0, "no 'features' array found"));
            }
        };
        let mut p = start;
        while p < hay.len() && hay[p] != b'[' {
            p += 1;
        }
        if hay.get(p) != Some(&b'[') {
            return Err(GeoError::geojson(start, "features is not an array"));
        }
        Ok(Self { bytes, pos: p + 1, at_end: false })
    }

    /// Return the next feature, or `None` when the array is exhausted.
    ///
    /// Returns `Some(Err(..))` for malformed features; iteration should stop after
    /// an error.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<Feature, GeoError>> {
        if self.at_end {
            return None;
        }
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' | b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.at_end = true;
                    return None;
                }
                b'{' => break,
                _ => {
                    self.at_end = true;
                    return None;
                }
            }
        }
        if self.pos >= self.bytes.len() {
            self.at_end = true;
            return None;
        }
        let start = self.pos;
        let end = match find_object_end(self.bytes, start) {
            Some(e) => e,
            None => {
                self.at_end = true;
                return Some(Err(GeoError::geojson(start, "unterminated feature object")));
            }
        };
        self.pos = end + 1;
        let slice = &self.bytes[start..=end];
        let json = match parse(slice) {
            Ok(j) => j,
            Err(e) => return Some(Err(GeoError::from(e))),
        };
        match parse_feature_json(&json) {
            Ok(f) => Some(Ok(f)),
            Err(e) => Some(Err(e)),
        }
    }

    /// Consume the stream, collecting every feature into a vector.
    pub fn collect(&mut self) -> Result<Vec<Feature>, GeoError> {
        let mut out = Vec::new();
        while let Some(r) = self.next() {
            out.push(r?);
        }
        Ok(out)
    }
}

/// A streaming Shapefile record reader over a `.shp` byte buffer.
pub struct ShapefileRecordStream<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ShapefileRecordStream<'a> {
    /// Create a new streaming reader at the first record (past the 100-byte header).
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 100 }
    }

    /// Return the next record, or `None` when the file is exhausted.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<ShapeRecord, GeoError>> {
        if self.pos + 8 > self.bytes.len() {
            return None;
        }
        match parse_shp_record_at(self.bytes, self.pos) {
            Ok((record, next_pos)) => {
                self.pos = next_pos;
                Some(Ok(record))
            }
            Err(e) => {
                self.pos = self.bytes.len();
                Some(Err(e))
            }
        }
    }

    /// Consume the stream, collecting all records.
    pub fn collect(&mut self) -> Result<Vec<ShapeRecord>, GeoError> {
        let mut out = Vec::new();
        while let Some(r) = self.next() {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_features() {
        let src = br#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":{}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[3,4]},"properties":{}}
        ]}"#;
        let mut s = GeoJsonFeatureStream::new(src).unwrap();
        let mut count = 0;
        while let Some(Ok(_)) = s.next() {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn stream_nested_array_in_feature() {
        let src = br#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]},"properties":{"a":[1,2,3]}}
        ]}"#;
        let mut s = GeoJsonFeatureStream::new(src).unwrap();
        let f = s.next().unwrap().unwrap();
        assert!(f.geometry.is_some());
    }

    #[test]
    fn stream_shapefile() {
        let shp = build_simple_polygon_shp();
        let mut s = ShapefileRecordStream::new(&shp);
        let r = s.next().unwrap().unwrap();
        assert_eq!(r.record_number, 1);
        let r2 = s.next();
        assert!(r2.is_none());
    }

    fn build_simple_polygon_shp() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&9994i32.to_be_bytes());
        for _ in 0..5 {
            b.extend_from_slice(&0i32.to_be_bytes());
        }
        let content_words = 64i32;
        let file_len = 50 + 4 + content_words;
        b.extend_from_slice(&(file_len as u32).to_be_bytes());
        b.extend_from_slice(&1000i32.to_le_bytes());
        b.extend_from_slice(&5i32.to_le_bytes());
        for v in [0.0f64, 0.0, 10.0, 10.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..4 {
            b.extend_from_slice(&0.0f64.to_le_bytes());
        }
        b.extend_from_slice(&1i32.to_be_bytes());
        b.extend_from_slice(&content_words.to_be_bytes());
        b.extend_from_slice(&5i32.to_le_bytes());
        for v in [0.0f64, 0.0, 10.0, 10.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&5i32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        for (x, y) in [
            (0.0f64, 0.0f64),
            (10.0f64, 0.0f64),
            (10.0f64, 10.0f64),
            (0.0f64, 10.0f64),
            (0.0f64, 0.0f64),
        ] {
            b.extend_from_slice(&x.to_le_bytes());
            b.extend_from_slice(&y.to_le_bytes());
        }
        b
    }
}
