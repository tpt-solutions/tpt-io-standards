//! Coordinate format and fixed-point conversion for Gerber (RS-274X) and
//! Excellon drill files.
//!
//! Gerber encodes coordinates as integers with an implied decimal point whose
//! position is fixed by the `FS` (format statement). For example `FSLAX36Y36`
//! places the decimal point 6 digits from the right. Whether leading or
//! trailing zeros are suppressed changes how those integers scale:
//!
//! * **Leading suppression** (right-justified): `value / 10^decimals`.
//! * **Trailing suppression** (left-justified): `value / 10^(total - decimals)`.

use alloc::string::ToString;

use crate::error::GerberError;

/// Axis selector for format/coordinate access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// X axis.
    X,
    /// Y axis.
    Y,
}

/// How insignificant zeros are elided from coordinate integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZeroSuppression {
    /// Leading (most-significant) zeros are removed; values are right-justified.
    #[default]
    Leading,
    /// Trailing (least-significant) zeros are removed; values are left-justified.
    Trailing,
}

/// Whether coordinates are absolute or incremental from the previous point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Notation {
    /// Each coordinate is relative to the origin.
    #[default]
    Absolute,
    /// Each coordinate is a delta from the previous point.
    Incremental,
}

/// The `FS` coordinate format declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinateFormat {
    /// Zero-suppression mode.
    pub zeros: ZeroSuppression,
    /// Absolute vs. incremental notation.
    pub notation: Notation,
    /// Number of integer digits in the X field.
    pub x_int: u8,
    /// Number of decimal digits in the X field.
    pub x_dec: u8,
    /// Number of integer digits in the Y field.
    pub y_int: u8,
    /// Number of decimal digits in the Y field.
    pub y_dec: u8,
}

impl Default for CoordinateFormat {
    fn default() -> Self {
        CoordinateFormat {
            zeros: ZeroSuppression::Leading,
            notation: Notation::Absolute,
            x_int: 3,
            x_dec: 6,
            y_int: 3,
            y_dec: 6,
        }
    }
}

impl CoordinateFormat {
    /// Total number of digits (integer + decimal) in the X field.
    pub fn x_total(&self) -> i32 {
        self.x_int as i32 + self.x_dec as i32
    }

    /// Total number of digits (integer + decimal) in the Y field.
    pub fn y_total(&self) -> i32 {
        self.y_int as i32 + self.y_dec as i32
    }

    /// Parse a format statement such as `FSLAX36Y36` (the leading `FS` and the
    /// trailing `*` are optional — pass only the inner code).
    pub fn parse(s: &str) -> Result<Self, GerberError> {
        let b = s.as_bytes();
        if b.len() < 2 || !b[..2].eq_ignore_ascii_case(b"FS") {
            return Err(GerberError::invalid_command(0, s.to_string()));
        }
        let mut i = 2usize;
        let zeros = match b.get(i) {
            Some(&c) => match c.to_ascii_uppercase() {
                b'L' => ZeroSuppression::Leading,
                b'T' => ZeroSuppression::Trailing,
                _ => return Err(GerberError::invalid_command(i, s.to_string())),
            },
            None => return Err(GerberError::invalid_command(0, s.to_string())),
        };
        i += 1;
        let notation = match b.get(i) {
            Some(&c) => match c.to_ascii_uppercase() {
                b'A' => Notation::Absolute,
                b'I' => Notation::Incremental,
                _ => return Err(GerberError::invalid_command(i, s.to_string())),
            },
            None => return Err(GerberError::invalid_command(0, s.to_string())),
        };
        i += 1;

        let mut x_int = 0u8;
        let mut x_dec = 0u8;
        let mut y_int = 0u8;
        let mut y_dec = 0u8;
        while i < b.len() {
            let c = b[i].to_ascii_uppercase();
            i += 1;
            let (intv, decv) = match (b.get(i), b.get(i + 1)) {
                (Some(&a), Some(&d)) if a.is_ascii_digit() && d.is_ascii_digit() => {
                    (a - b'0', d - b'0')
                }
                _ => return Err(GerberError::invalid_command(i, s.to_string())),
            };
            i += 2;
            match c {
                b'X' => {
                    x_int = intv;
                    x_dec = decv;
                }
                b'Y' => {
                    y_int = intv;
                    y_dec = decv;
                }
                _ => return Err(GerberError::invalid_command(i, s.to_string())),
            }
        }
        Ok(CoordinateFormat { zeros, notation, x_int, x_dec, y_int, y_dec })
    }

    /// Scale a raw integer coordinate on `axis` to its real-world value.
    pub fn scale_axis(&self, axis: Axis, value: i64) -> f64 {
        let (total, dec) = match axis {
            Axis::X => (self.x_total(), self.x_dec as i32),
            Axis::Y => (self.y_total(), self.y_dec as i32),
        };
        let factor_exp = match self.zeros {
            ZeroSuppression::Leading => dec,
            ZeroSuppression::Trailing => total - dec,
        };
        value as f64 / 10f64.powi(factor_exp)
    }

    /// Inverse of [`CoordinateFormat::scale_axis`]: turn a real-world value back
    /// into the integer encoding used by this format (rounded to nearest).
    pub fn encode_axis(&self, axis: Axis, value: f64) -> i64 {
        let (total, dec) = match axis {
            Axis::X => (self.x_total(), self.x_dec as i32),
            Axis::Y => (self.y_total(), self.y_dec as i32),
        };
        let factor_exp = match self.zeros {
            ZeroSuppression::Leading => dec,
            ZeroSuppression::Trailing => total - dec,
        };
        (value * 10f64.powi(factor_exp)).round() as i64
    }

    /// Whether `value` can be written on `axis` in this format.
    ///
    /// The encoded integer has `int + dec` digits, so its magnitude must stay below
    /// `10^(int + dec)`. Non-finite values never fit.
    pub fn fits_axis(&self, axis: Axis, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let total = match axis {
            Axis::X => self.x_total(),
            Axis::Y => self.y_total(),
        };
        let limit = 10i64.checked_pow(total as u32).unwrap_or(i64::MAX);
        self.encode_axis(axis, value).unsigned_abs() < limit as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_format() {
        let f = CoordinateFormat::parse("FSLAX36Y36").unwrap();
        assert_eq!(f.zeros, ZeroSuppression::Leading);
        assert_eq!(f.notation, Notation::Absolute);
        assert_eq!((f.x_int, f.x_dec), (3, 6));
        assert_eq!((f.y_int, f.y_dec), (3, 6));
    }

    #[test]
    fn parse_format_trailing_incremental() {
        let f = CoordinateFormat::parse("FSTIX24Y24").unwrap();
        assert_eq!(f.zeros, ZeroSuppression::Trailing);
        assert_eq!(f.notation, Notation::Incremental);
    }

    #[test]
    fn leading_suppression_scales_by_decimals() {
        let f = CoordinateFormat::parse("FSLAX36Y36").unwrap();
        // 1.500 mm encoded as 1500000 with 6 decimals.
        assert!((f.scale_axis(Axis::X, 1_500_000) - 1.5).abs() < 1e-9);
        assert_eq!(f.encode_axis(Axis::X, 1.5), 1_500_000);
    }

    #[test]
    fn trailing_suppression_scales_by_total_minus_decimals() {
        let f = CoordinateFormat::parse("FSTAX24Y24").unwrap();
        // 2-int / 4-dec, trailing suppression: 12.34 -> "1234".
        assert!((f.scale_axis(Axis::X, 1234) - 12.34).abs() < 1e-9);
        assert_eq!(f.encode_axis(Axis::X, 12.34), 1234);
    }

    #[test]
    fn round_trip_is_stable() {
        let f = CoordinateFormat::parse("FSLAX36Y36").unwrap();
        for v in [0.0_f64, 1.5, -2.25, 100.0, -0.000123] {
            let enc = f.encode_axis(Axis::X, v);
            let dec = f.scale_axis(Axis::X, enc);
            assert!((dec - v).abs() < 1e-9, "v={v}, enc={enc}, dec={dec}");
        }
    }
}
