//! SPK segment evaluation: Chebyshev polynomials for data types 2 and 3.
//!
//! A type 2 segment holds, for each time interval, the Chebyshev coefficients of the x, y and z position components.
//! A type 3 segment also holds coefficients for the velocity components. Each record starts with its midpoint and
//! radius, and the segment ends with four words: the first epoch (INIT), the interval length (INTLEN), the words per
//! record (RSIZE) and the record count (N). Velocity for type 2 comes from differentiating the position polynomials.

use alloc::string::ToString;

use crate::{Daf, NaifError, Summary};

/// A state: position in kilometres and velocity in kilometres per second, relative to the segment's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    /// Position (km).
    pub position: [f64; 3],
    /// Velocity (km/s).
    pub velocity: [f64; 3],
}

/// The state of `target` at epoch `et`, from the first segment whose coverage includes both.
pub fn state(daf: &Daf<'_>, target: i32, et: f64) -> Result<State, NaifError> {
    let s = daf
        .summaries
        .iter()
        .find(|s| s.target == target && s.start_et <= et && et <= s.end_et)
        .ok_or(NaifError::NoSegment)?;
    evaluate(daf, s, et)
}

/// Evaluate one segment at epoch `et`.
pub fn evaluate(daf: &Daf<'_>, s: &Summary, et: f64) -> Result<State, NaifError> {
    let data = daf.segment(s)?;
    if data.len() < 5 {
        return Err(NaifError::Invalid("segment is too short for its trailer".to_string()));
    }
    let n = data.len();
    let init = data[n - 4];
    let intlen = data[n - 3];
    let rsize = data[n - 2] as usize;
    let count = data[n - 1] as usize;
    if intlen <= 0.0 || count == 0 || rsize < 5 || rsize.saturating_mul(count) + 4 > n {
        return Err(NaifError::Invalid(
            "segment trailer does not describe its records".to_string(),
        ));
    }
    // Index of the interval holding `et`, clamped to the segment's records. The float-to-int cast truncates toward
    // zero, which keeps this free of `std` rounding functions and gives 0 for epochs before the first interval.
    let index = (((et - init) / intlen) as isize).clamp(0, count as isize - 1) as usize;
    let rec = &data[index * rsize..(index + 1) * rsize];
    let mid = rec[0];
    let radius = rec[1];
    let x = (et - mid) / radius;
    let coeffs = &rec[2..];
    match s.data_type {
        2 => {
            let n = coeffs.len() / 3;
            let mut position = [0.0; 3];
            let mut velocity = [0.0; 3];
            for c in 0..3 {
                let (value, slope) = chebyshev(&coeffs[c * n..(c + 1) * n], x);
                position[c] = value;
                velocity[c] = slope / radius;
            }
            Ok(State { position, velocity })
        }
        3 => {
            let n = coeffs.len() / 6;
            let mut position = [0.0; 3];
            let mut velocity = [0.0; 3];
            for c in 0..3 {
                position[c] = chebyshev(&coeffs[c * n..(c + 1) * n], x).0;
                velocity[c] = chebyshev(&coeffs[(c + 3) * n..(c + 4) * n], x).0;
            }
            Ok(State { position, velocity })
        }
        _ => Err(NaifError::Unsupported("SPK segment types other than 2 and 3")),
    }
}

/// The value and derivative of the Chebyshev series `sum c_k T_k(x)` at `x`, from the three-term recurrences
/// `T_k = 2x T_(k-1) - T_(k-2)` and `T_k' = 2 T_(k-1) + 2x T_(k-1)' - T_(k-2)'`.
fn chebyshev(coef: &[f64], x: f64) -> (f64, f64) {
    let mut value = 0.0;
    let mut slope = 0.0;
    if let Some(&c0) = coef.first() {
        value += c0;
    }
    if let Some(&c1) = coef.get(1) {
        value += c1 * x;
        slope += c1;
    }
    let (mut t_prev, mut t_cur) = (1.0, x);
    let (mut d_prev, mut d_cur) = (0.0, 1.0);
    for &c in coef.iter().skip(2) {
        let t_next = 2.0 * x * t_cur - t_prev;
        let d_next = 2.0 * t_cur + 2.0 * x * d_cur - d_prev;
        value += c * t_next;
        slope += c * d_next;
        t_prev = t_cur;
        t_cur = t_next;
        d_prev = d_cur;
        d_cur = d_next;
    }
    (value, slope)
}
