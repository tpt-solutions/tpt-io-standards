//! Depth-alignment utilities for correlating multiple well-log runs.
//!
//! Different logging passes share an index curve (usually `DEPT`) but sample at
//! different increments or start/stop depths. [`align_to`] resamples a source
//! log onto a target index using nearest-neighbour lookup, so curves from
//! different runs can be compared row-for-row.

use crate::model::WellLog;

/// Linear-interpolate `value_at` sampled at `xs`, evaluated at `x`.
///
/// Returns `None` if `x` is outside `[xs[0], xs[last]]]` or `xs` is empty. The
/// `xs` series must be monotonic.
fn interp(xs: &[f64], value_at: impl Fn(usize) -> f64, x: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    if x <= xs[0] {
        return Some(value_at(0));
    }
    if x >= xs[xs.len() - 1] {
        return Some(value_at(xs.len() - 1));
    }
    // Binary search for the largest index `i` with `xs[i] <= x`.
    let mut lo = 0usize;
    let mut hi = xs.len() - 1;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if xs[mid] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let x0 = xs[lo];
    let x1 = xs[hi];
    let v0 = value_at(lo);
    let v1 = value_at(hi);
    if x1 == x0 {
        return Some(v0);
    }
    let t = (x - x0) / (x1 - x0);
    Some(v0 + t * (v1 - v0))
}

/// Resample `source` onto the index (depth) values of `target`.
///
/// Returns a new [`WellLog`] whose index curve matches `target` exactly and
/// whose other channels are interpolated from `source`. The result keeps each
/// log's curve definitions and header metadata from `target`; source channels
/// not present in `target` are dropped, and target channels not present in
/// `source` are filled with `target.well.null_value`.
///
/// `source` and `target` must each have at least one curve (the index).
pub fn align_to(target: &WellLog, source: &WellLog) -> Option<WellLog> {
    if target.curves.is_empty() || source.curves.is_empty() {
        return None;
    }
    let t_index = target.index_values().collect::<alloc::vec::Vec<f64>>();
    let s_index = source.index_values().collect::<alloc::vec::Vec<f64>>();

    let mut data = alloc::vec::Vec::with_capacity(t_index.len());
    for &depth in &t_index {
        let mut row = alloc::vec::Vec::with_capacity(target.curves.len());
        for ci in 0..target.curves.len() {
            if ci == 0 {
                row.push(depth);
                continue;
            }
            let mnem = &target.curves[ci].mnemonic;
            // Find the matching source channel by mnemonic.
            let src_col = source.curves.iter().position(|c| c.mnemonic == *mnem);
            match src_col {
                Some(sc) => {
                    let v = interp(&s_index, |i| source.data[i][sc], depth)
                        .unwrap_or(target.well.null_value);
                    row.push(v);
                }
                None => row.push(target.well.null_value),
            }
        }
        data.push(row);
    }

    Some(WellLog {
        version: target.version,
        wrap: target.wrap,
        well: target.well.clone(),
        parameters: target.parameters.clone(),
        curves: target.curves.clone(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;

    const A: &str = "\
~Version Information
VERS. 2.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
~Curve Information
DEPT.M   : DEPTH
GR.API   : GAMMA RAY
~ASCII Log Data
10.0 75.5
20.0 80.1
30.0 78.4
";

    const B: &str = "\
~Version Information
VERS. 2.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
~Curve Information
DEPT.M   : DEPTH
GR.API   : GAMMA RAY
~ASCII Log Data
15.0 77.0
25.0 79.0
";

    #[test]
    fn aligns_onto_target_depths() {
        let a = parse_str(A).unwrap();
        let b = parse_str(B).unwrap();
        let aligned = align_to(&a, &b).unwrap();
        // Target index is preserved exactly.
        assert_eq!(aligned.index_values().collect::<Vec<_>>(), vec![10.0, 20.0, 30.0]);
        // At depth 10 (== b's start) GR should equal b's first value (77.0).
        assert!((aligned.data[0][1] - 77.0).abs() < 1e-6);
    }

    #[test]
    fn interpolate_between_samples() {
        let a = parse_str(A).unwrap();
        let b = parse_str(B).unwrap();
        let aligned = align_to(&a, &b).unwrap();
        // Depth 20 is midway between b's 15 (77.0) and 25 (79.0) → ~78.0.
        assert!((aligned.data[1][1] - 78.0).abs() < 1e-6);
    }
}
