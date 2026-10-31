//! Touchstone parsing, conversion to real/imaginary form, and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::TouchstoneError;

/// A complex number.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

/// The network parameter type from the option line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    /// Scattering parameters (default).
    S,
    /// Admittance parameters.
    Y,
    /// Impedance parameters.
    Z,
    /// Hybrid-h parameters.
    H,
    /// Hybrid-g parameters.
    G,
}

/// The number format of the data values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Magnitude and angle in degrees.
    Ma,
    /// Decibels (20·log10 magnitude) and angle in degrees.
    Db,
    /// Real and imaginary parts.
    Ri,
}

/// One frequency point with its N×N matrix, `matrix[row][col]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// Frequency in hertz.
    pub freq_hz: f64,
    /// The network matrix in real/imaginary form.
    pub matrix: Vec<Vec<Complex>>,
}

/// A Touchstone file.
#[derive(Debug, Clone, PartialEq)]
pub struct Touchstone {
    /// Number of ports N.
    pub ports: usize,
    /// The parameter type from the option line.
    pub param: Param,
    /// The reference impedance in ohms.
    pub reference_ohm: f64,
    /// The data points in file order.
    pub points: Vec<Point>,
}

fn unit_scale(u: &str) -> Option<f64> {
    match u.to_ascii_uppercase().as_str() {
        "HZ" => Some(1.0),
        "KHZ" => Some(1e3),
        "MHZ" => Some(1e6),
        "GHZ" => Some(1e9),
        _ => None,
    }
}

/// Parse a Touchstone file with `ports` ports (taken from the `.sNp` extension).
pub fn parse(input: &[u8], ports: usize) -> Result<Touchstone, TouchstoneError> {
    if ports == 0 {
        return Err(TouchstoneError::Invalid("port count must be at least 1".to_string()));
    }
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut freq_scale = 1e9;
    let mut param = Param::S;
    let mut format = Format::Ma;
    let mut reference_ohm = 50.0;
    let mut seen_option = false;
    let mut numbers: Vec<f64> = Vec::new();
    let mut points = Vec::new();
    let per_point = 1 + 2 * ports * ports;

    for (lineno, raw) in text.lines().enumerate() {
        let code = raw.split('!').next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        if let Some(opts) = code.strip_prefix('#') {
            if seen_option {
                return Err(TouchstoneError::Invalid("more than one option line".to_string()));
            }
            seen_option = true;
            let tokens: Vec<&str> = opts.split_whitespace().collect();
            let mut i = 0;
            while i < tokens.len() {
                let t = tokens[i].to_ascii_uppercase();
                if let Some(scale) = unit_scale(&t) {
                    freq_scale = scale;
                } else {
                    match t.as_str() {
                        "S" => param = Param::S,
                        "Y" => param = Param::Y,
                        "Z" => param = Param::Z,
                        "H" => param = Param::H,
                        "G" => param = Param::G,
                        "MA" => format = Format::Ma,
                        "DB" => format = Format::Db,
                        "RI" => format = Format::Ri,
                        "R" => {
                            i += 1;
                            let r = tokens.get(i).ok_or_else(|| {
                                TouchstoneError::Invalid("R needs a value".to_string())
                            })?;
                            reference_ohm = r
                                .parse::<f64>()
                                .map_err(|_| ParseError::custom(0, "bad reference impedance"))?;
                        }
                        _ => {
                            return Err(TouchstoneError::Invalid(alloc::format!(
                                "unknown option '{t}'"
                            )))
                        }
                    }
                }
                i += 1;
            }
            continue;
        }
        for tok in code.split_whitespace() {
            let v =
                tok.parse::<f64>().map_err(|_| ParseError::custom(lineno, "expected a number"))?;
            numbers.push(v);
        }
        while numbers.len() >= per_point {
            let chunk: Vec<f64> = numbers.drain(..per_point).collect();
            points.push(point(&chunk, freq_scale, format, ports));
        }
    }
    if !numbers.is_empty() {
        return Err(TouchstoneError::Invalid("data ends in the middle of a point".to_string()));
    }
    Ok(Touchstone { ports, param, reference_ohm, points })
}

/// Build one point from a frequency and `2·N²` numbers in file order.
fn point(v: &[f64], freq_scale: f64, format: Format, ports: usize) -> Point {
    let mut matrix = alloc::vec![alloc::vec![Complex::default(); ports]; ports];
    // Two-port files list the matrix as S11 S21 S12 S22 (column by column); larger files list
    // each row in turn (S11 S12 S13 … S21 …).
    for (k, pair) in v[1..].chunks_exact(2).enumerate() {
        let (row, col) = if ports == 2 { (k % 2, k / 2) } else { (k / ports, k % ports) };
        matrix[row][col] = to_complex(pair[0], pair[1], format);
    }
    Point { freq_hz: v[0] * freq_scale, matrix }
}

fn to_complex(a: f64, b: f64, format: Format) -> Complex {
    match format {
        Format::Ri => Complex { re: a, im: b },
        Format::Ma => polar(a, b),
        Format::Db => polar(10f64.powf(a / 20.0), b),
    }
}

fn polar(mag: f64, angle_deg: f64) -> Complex {
    let a = angle_deg.to_radians();
    Complex { re: mag * libm_cos(a), im: mag * libm_sin(a) }
}

// `f64::sin` and `f64::cos` are not available without `std`, so the crate evaluates the Taylor
// series on an argument reduced to [-π, π]. Sixteen terms reach full double precision there.
fn reduce(x: f64) -> f64 {
    let tau = core::f64::consts::TAU;
    let mut y = x % tau;
    if y > core::f64::consts::PI {
        y -= tau;
    } else if y < -core::f64::consts::PI {
        y += tau;
    }
    y
}

fn libm_sin(x: f64) -> f64 {
    let y = reduce(x);
    let mut term = y;
    let mut sum = y;
    for n in 1..16 {
        term *= -y * y / ((2 * n) as f64 * (2 * n + 1) as f64);
        sum += term;
    }
    sum
}

fn libm_cos(x: f64) -> f64 {
    let y = reduce(x);
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..16 {
        term *= -y * y / ((2 * n - 1) as f64 * (2 * n) as f64);
        sum += term;
    }
    sum
}

/// Serialise a Touchstone file in RI form with a `GHz`-free `Hz` option line, so frequencies
/// round-trip exactly. The matrix is written row-major, with `N` values per line after the first
/// line of each point.
pub fn write(t: &Touchstone) -> Vec<u8> {
    let p = match t.param {
        Param::S => 'S',
        Param::Y => 'Y',
        Param::Z => 'Z',
        Param::H => 'H',
        Param::G => 'G',
    };
    let mut out = String::new();
    let _ = writeln!(out, "# Hz {p} RI R {}", t.reference_ohm);
    for pt in &t.points {
        let n = pt.matrix.len();
        let mut cells: Vec<String> = alloc::vec![alloc::format!("{}", pt.freq_hz)];
        // Mirror the reader: two-port files are column-major, larger files are row-major.
        let order: Vec<(usize, usize)> = if n == 2 {
            (0..n).flat_map(|c| (0..n).map(move |r| (r, c))).collect()
        } else {
            (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).collect()
        };
        for (r, c) in order {
            let v = pt.matrix[r][c];
            cells.push(alloc::format!("{}", v.re));
            cells.push(alloc::format!("{}", v.im));
        }
        let _ = writeln!(out, "{}", cells.join(" "));
    }
    out.into_bytes()
}
