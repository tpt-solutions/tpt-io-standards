//! TLE field layout, parsing and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::TleError;

/// One element set: an optional name and the orbital elements from its two lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Tle {
    /// The name line, trimmed, if the set has one.
    pub name: Option<String>,
    /// NORAD catalogue number (columns 3–7 of both lines).
    pub satellite_number: u32,
    /// Classification: `U`, `C` or `S`.
    pub classification: char,
    /// International designator, e.g. `98067A`.
    pub international_designator: String,
    /// Full epoch year (the two-digit field maps 57–99 to 19xx and 00–56 to 20xx).
    pub epoch_year: u16,
    /// Day of the year with fraction; 1.0 is 00:00 on 1 January.
    pub epoch_day: f64,
    /// First derivative of mean motion, revolutions per day squared (columns 34–43).
    pub mean_motion_dot: f64,
    /// Second derivative of mean motion (columns 45–52).
    pub mean_motion_ddot: f64,
    /// B* drag term, 1/Earth radii (columns 54–61).
    pub bstar: f64,
    /// Ephemeris type digit (column 63).
    pub ephemeris_type: u8,
    /// Element set number (columns 65–68).
    pub element_set_number: u32,
    /// Inclination, degrees.
    pub inclination: f64,
    /// Right ascension of the ascending node, degrees.
    pub raan: f64,
    /// Eccentricity, between 0 and 1.
    pub eccentricity: f64,
    /// Argument of perigee, degrees.
    pub argument_of_perigee: f64,
    /// Mean anomaly, degrees.
    pub mean_anomaly: f64,
    /// Mean motion, revolutions per day.
    pub mean_motion: f64,
    /// Revolution number at epoch (columns 64–68 of line 2).
    pub revolution_number: u32,
}

/// Modulo-10 checksum of the first 68 columns of a line: digits add their value, `-` adds 1.
pub fn checksum(line: &str) -> u32 {
    let sum: u32 = line
        .bytes()
        .take(68)
        .map(|b| match b {
            b'0'..=b'9' => (b - b'0') as u32,
            b'-' => 1,
            _ => 0,
        })
        .sum();
    sum % 10
}

/// Parse every TLE in a file. Blank lines are ignored.
pub fn parse(input: &[u8]) -> Result<Vec<Tle>, TleError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "TLE file is not valid UTF-8"))?;
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end()).filter(|l| !l.is_empty()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (name, l1_index) = if lines[i].starts_with("1 ") {
            (None, i)
        } else {
            (Some(lines[i].trim().to_string()), i + 1)
        };
        let l1 = lines.get(l1_index).copied().ok_or_else(|| ParseError::eof(text.len()))?;
        let l2 = lines.get(l1_index + 1).copied().ok_or_else(|| ParseError::eof(text.len()))?;
        if !l1.starts_with("1 ") || !l2.starts_with("2 ") {
            return Err(ParseError::custom(0, "expected a line 1 and line 2 pair").into());
        }
        out.push(elements(name, l1, l2)?);
        i = l1_index + 2;
    }
    Ok(out)
}

/// Serialise element sets as TLE text: each set is written as its name line (if any) and the two
/// fixed-width lines with recomputed checksums.
pub fn write(sets: &[Tle]) -> Vec<u8> {
    let mut out = String::new();
    for t in sets {
        if let Some(n) = &t.name {
            let _ = writeln!(out, "{n}");
        }
        let l1 = line1(t);
        let l2 = line2(t);
        let _ = writeln!(out, "{}{}", l1, checksum(&l1));
        let _ = writeln!(out, "{}{}", l2, checksum(&l2));
    }
    out.into_bytes()
}

fn field(line: &str, from: usize, to: usize) -> &str {
    line.get(from - 1..to).unwrap_or("").trim()
}

fn number(s: &str, what: &str) -> Result<f64, TleError> {
    let fixed = match s.strip_prefix('-') {
        Some(rest) if rest.starts_with('.') => alloc::format!("-0{rest}"),
        _ if s.starts_with('.') => alloc::format!("0{s}"),
        _ => s.to_string(),
    };
    fixed.parse::<f64>().map_err(|_| TleError::Invalid(alloc::format!("{what} is not a number")))
}

/// Exponent form: `[sign]DDDDD[sign]D`, read as `0.DDDDD × 10^±D`.
fn exponent_form(s: &str, what: &str) -> Result<f64, TleError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0.0);
    }
    let (sign, body) = match s.as_bytes()[0] {
        b'-' => ("-", &s[1..]),
        b'+' => ("", &s[1..]),
        _ => ("", s),
    };
    if body.len() < 2 {
        return Err(TleError::Invalid(alloc::format!("{what} is malformed")));
    }
    let (mantissa, exp) = body.split_at(body.len() - 2);
    let exp = alloc::format!("{}{}", &exp[..1], &exp[1..]);
    let v = alloc::format!("{sign}0.{mantissa}e{exp}");
    v.parse::<f64>().map_err(|_| TleError::Invalid(alloc::format!("{what} is malformed")))
}

fn int(s: &str, what: &str) -> Result<u32, TleError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| TleError::Invalid(alloc::format!("{what} is not an integer")))
}

fn elements(name: Option<String>, l1: &str, l2: &str) -> Result<Tle, TleError> {
    for (n, line) in [(1u8, l1), (2u8, l2)] {
        if line.len() < 69 {
            return Err(ParseError::custom(0, "TLE line is shorter than 69 columns").into());
        }
        let got = line.as_bytes()[68];
        let expect = checksum(line);
        if (got as char).to_digit(10) != Some(expect) {
            let sat = int(field(line, 3, 7), "satellite number").unwrap_or(0);
            return Err(TleError::Checksum { satellite: sat, line: n });
        }
    }
    let sat1 = int(field(l1, 3, 7), "satellite number")?;
    let sat2 = int(field(l2, 3, 7), "satellite number")?;
    if sat1 != sat2 {
        return Err(TleError::Invalid("satellite numbers differ between lines".to_string()));
    }
    let yy = int(field(l1, 19, 20), "epoch year")?;
    let year = if yy < 57 { 2000 + yy } else { 1900 + yy } as u16;
    let ecc_digits = field(l2, 27, 33);
    let eccentricity = number(&alloc::format!("0.{ecc_digits}"), "eccentricity")?;
    let t = Tle {
        name,
        satellite_number: sat1,
        classification: l1.as_bytes()[7] as char,
        international_designator: field(l1, 10, 17).to_string(),
        epoch_year: year,
        epoch_day: number(field(l1, 21, 32), "epoch day")?,
        mean_motion_dot: number(field(l1, 34, 43), "mean motion derivative")?,
        mean_motion_ddot: exponent_form(&l1[44..52], "second derivative")?,
        bstar: exponent_form(&l1[53..61], "B*")?,
        ephemeris_type: int(field(l1, 63, 63), "ephemeris type")? as u8,
        element_set_number: int(field(l1, 65, 68), "element set number")?,
        inclination: number(field(l2, 9, 16), "inclination")?,
        raan: number(field(l2, 18, 25), "RAAN")?,
        eccentricity,
        argument_of_perigee: number(field(l2, 35, 42), "argument of perigee")?,
        mean_anomaly: number(field(l2, 44, 51), "mean anomaly")?,
        mean_motion: number(field(l2, 53, 63), "mean motion")?,
        revolution_number: int(field(l2, 64, 68), "revolution number")?,
    };
    validate(&t)?;
    Ok(t)
}

fn validate(t: &Tle) -> Result<(), TleError> {
    if !(0.0..1.0).contains(&t.eccentricity) {
        return Err(TleError::Invalid("eccentricity must be in [0, 1)".to_string()));
    }
    if !(0.0..=180.0).contains(&t.inclination) {
        return Err(TleError::Invalid("inclination must be in [0, 180] degrees".to_string()));
    }
    if t.mean_motion.is_nan() || t.mean_motion <= 0.0 {
        return Err(TleError::Invalid("mean motion must be positive".to_string()));
    }
    if !(1.0..367.0).contains(&t.epoch_day) {
        return Err(TleError::Invalid("epoch day must be in [1, 367)".to_string()));
    }
    Ok(())
}

/// Format a value as `±.DDDDDDDD` in 10 columns (the first-derivative field).
fn dot_field(v: f64) -> String {
    let sign = if v < 0.0 { '-' } else { ' ' };
    let body = alloc::format!("{:.8}", v.abs());
    let body = body.strip_prefix('0').unwrap_or(&body);
    alloc::format!("{sign}{body:>9}")
}

/// Format a value in the 8-column exponent form `±DDDDD±D`.
fn exp_field(v: f64) -> String {
    if v == 0.0 {
        return " 00000-0".to_string();
    }
    let sign = if v < 0.0 { '-' } else { ' ' };
    let a = v.abs();
    let mut e = a.log10().floor() as i32 + 1;
    let mut mant = (a / 10f64.powi(e) * 1e5).round() as u32;
    if mant >= 100_000 {
        mant /= 10;
        e += 1;
    }
    let esign = if e < 0 { '-' } else { '+' };
    alloc::format!("{sign}{mant:05}{esign}{}", e.abs())
}

fn line1(t: &Tle) -> String {
    let yy = t.epoch_year % 100;
    alloc::format!(
        "1 {:>5}{} {:<8} {:02}{:012.8} {} {} {} {} {:>4}",
        t.satellite_number,
        t.classification,
        t.international_designator,
        yy,
        t.epoch_day,
        dot_field(t.mean_motion_dot),
        exp_field(t.mean_motion_ddot),
        exp_field(t.bstar),
        t.ephemeris_type,
        t.element_set_number
    )
}

fn line2(t: &Tle) -> String {
    let ecc = (t.eccentricity * 1e7).round() as u64;
    alloc::format!(
        "2 {:>5} {:>8.4} {:>8.4} {:07} {:>8.4} {:>8.4} {:>11.8}{:>5}",
        t.satellite_number,
        t.inclination,
        t.raan,
        ecc,
        t.argument_of_perigee,
        t.mean_anomaly,
        t.mean_motion,
        t.revolution_number
    )
}
