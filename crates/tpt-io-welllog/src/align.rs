//! Depth-alignment utilities for correlating multiple well-log runs.
//!
//! Different logging runs cover overlapping but non-identical depth ranges and
//! sample at different intervals. These helpers build a common depth grid,
//! resample each run onto it by linear interpolation, and offer a simple
//! cross-correlation to estimate the depth shift that best aligns two runs of
//! the same curve.

use alloc::vec::Vec;

use crate::model::WellLog;

/// A set of runs resampled onto a single, shared depth grid.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AlignedLogs {
    /// Common, monotonically increasing depth grid (column 0 of every run).
    pub depths: Vec<f64>,
    /// One entry per input run: `values[run][channel][sample]` of resampled
    /// channel readings aligned to [`AlignedLogs::depths`].
    pub values: Vec<Vec<Vec<f64>>>,
    /// The original (pre-alignment) start depth of each run, parallel to `values`.
    pub run_start: Vec<f64>,
    /// The original end depth of each run.
    pub run_end: Vec<f64>,
}

impl AlignedLogs {
    /// Number of aligned runs.
    pub fn run_count(&self) -> usize {
        self.values.len()
    }

    /// Number of depth samples on the common grid.
    pub fn sample_count(&self) -> usize {
        self.depths.len()
    }

    /// The resampled channel matrix for run `run`, or `None` if out of range.
    pub fn run(&self, run: usize) -> Option<&Vec<Vec<f64>>> {
        self.values.get(run)
    }
}

/// Compute a uniform depth grid spanning the union of all runs' depth ranges.
///
/// `step`, if `None`, defaults to the smallest nominal sample interval observed
/// across the runs (clamped to a positive value).
pub fn common_grid(runs: &[&WellLog], step: Option<f64>) -> Vec<f64> {
    if runs.is_empty() {
        return Vec::new();
    }
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut smallest_interval = f64::INFINITY;
    for r in runs {
        if r.is_empty() {
            continue;
        }
        let first = r.data[0][0];
        let last = r.data[r.data.len() - 1][0];
        min = min.min(first.min(last));
        max = max.max(first.max(last));
        for w in r.data.windows(2) {
            let d = (w[1][0] - w[0][0]).abs();
            if d > 0.0 {
                smallest_interval = smallest_interval.min(d);
            }
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return Vec::new();
    }
    let step =
        step.unwrap_or_else(|| if smallest_interval.is_finite() { smallest_interval } else { 1.0 });
    let step = if step <= 0.0 { 1.0 } else { step };
    let n = ((max - min) / step).ceil() as usize + 1;
    let mut grid = Vec::with_capacity(n);
    for i in 0..n {
        grid.push(min + step * i as f64);
    }
    grid
}

/// Linearly resample one run's channels onto `depths`.
///
/// Values at depths outside the run's range are clamped to the nearest endpoint
/// (no extrapolation). The index curve (depth) is not interpolated; instead the
/// returned matrix's column 0 is the requested `depths` value.
pub fn resample(log: &WellLog, depths: &[f64]) -> Vec<Vec<f64>> {
    let ncol = log.curves.len();
    let mut out: Vec<Vec<f64>> = Vec::with_capacity(depths.len());
    for &d in depths {
        let mut row = Vec::with_capacity(ncol);
        // Column 0 is the (common) depth itself.
        row.push(d);
        for c in 1..ncol {
            row.push(sample_channel(log, c, d));
        }
        out.push(row);
    }
    out
}

/// Sample channel `col` of `log` at depth `d` by linear interpolation.
pub fn sample_channel(log: &WellLog, col: usize, d: f64) -> f64 {
    if log.is_empty() {
        return 0.0;
    }
    let data = &log.data;
    if d <= data[0][0] {
        return data[0][col];
    }
    if d >= data[data.len() - 1][0] {
        return data[data.len() - 1][col];
    }
    // Binary search for the bracketing interval.
    let mut lo = 0usize;
    let mut hi = data.len() - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if data[mid][0] <= d {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (d0, v0) = (data[lo][0], data[lo][col]);
    let (d1, v1) = (data[hi][0], data[hi][col]);
    if d1 == d0 {
        return v0;
    }
    v0 + (v1 - v0) * (d - d0) / (d1 - d0)
}

/// Align several runs onto a common depth grid.
///
/// If `step` is `None` a grid step is inferred from the data (see
/// [`common_grid`]).
pub fn align_runs(runs: &[&WellLog], step: Option<f64>) -> AlignedLogs {
    let depths = common_grid(runs, step);
    let mut values = Vec::with_capacity(runs.len());
    let mut run_start = Vec::with_capacity(runs.len());
    let mut run_end = Vec::with_capacity(runs.len());
    for r in runs {
        run_start.push(r.data.first().map(|d| d[0]).unwrap_or(0.0));
        run_end.push(r.data.last().map(|d| d[0]).unwrap_or(0.0));
        values.push(resample(r, &depths));
    }
    AlignedLogs { depths, values, run_start, run_end }
}

/// Estimate the depth shift that best aligns channel `col` of run `b` to the
/// same channel of run `a`, over their overlapping depth range.
///
/// Returns the shift `s` such that `b`'s depth `x` corresponds to `a`'s depth
/// `x + s` (i.e. `b` should be shifted by `s` to match `a`). The search is
/// performed on a `step`-spaced lattice within `[-max_shift, +max_shift]`.
pub fn cross_correlate(a: &WellLog, b: &WellLog, col: usize, max_shift: f64, step: f64) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let step = if step <= 0.0 { 1.0 } else { step };
    let max_shift = if max_shift <= 0.0 { step } else { max_shift };

    let mut best_shift = 0.0f64;
    let mut best_corr = f64::NEG_INFINITY;

    let mut s = -max_shift;
    while s <= max_shift {
        let corr = correlate_at(a, b, col, s);
        if corr > best_corr {
            best_corr = corr;
            best_shift = s;
        }
        s += step;
    }
    best_shift
}

/// Pearson correlation between channel `col` of `a` and `b` shifted by `shift`.
fn correlate_at(a: &WellLog, b: &WellLog, col: usize, shift: f64) -> f64 {
    let mut n = 0usize;
    let mut sa = 0.0;
    let mut sb = 0.0;
    let mut saa = 0.0;
    let mut sbb = 0.0;
    let mut sab = 0.0;

    let a0 = a.data[0][0];
    let a1 = a.data[a.data.len() - 1][0];
    for row in &a.data {
        let da = row[0];
        if da < a0 || da > a1 {
            continue;
        }
        let db = da - shift;
        if db < b.data[0][0] || db > b.data[b.data.len() - 1][0] {
            continue;
        }
        let va = row[col];
        let vb = sample_channel(b, col, db);
        n += 1;
        sa += va;
        sb += vb;
        saa += va * va;
        sbb += vb * vb;
        sab += va * vb;
    }
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;
    let num = sab - sa * sb / nf;
    let den = ((saa - sa * sa / nf) * (sbb - sb * sb / nf)).sqrt();
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Build a run whose channel (col 1) is `f(depth)`; curves length matches
    /// the 2-column data.
    fn run(start: f64, step: f64, n: usize, f: impl Fn(f64) -> f64) -> WellLog {
        let mut data = Vec::new();
        for i in 0..n {
            let d = start + step * i as f64;
            data.push(vec![d, f(d)]);
        }
        WellLog {
            version: 2.0,
            wrap: false,
            well: Default::default(),
            parameters: Vec::new(),
            curves: vec![
                crate::model::Curve { mnemonic: "DEPT".into(), ..Default::default() },
                crate::model::Curve { mnemonic: "GR".into(), ..Default::default() },
            ],
            data,
        }
    }

    #[test]
    fn resample_linear_interpolates() {
        let log = run(0.0, 10.0, 5, |d| d * 2.0); // 0,10,20,30,40
        let depths = [5.0, 15.0, 25.0];
        let out = resample(&log, &depths);
        assert_eq!(out.len(), 3);
        assert!((out[0][1] - 10.0).abs() < 1e-9); // 5*2
        assert!((out[1][1] - 30.0).abs() < 1e-9); // 15*2
    }

    #[test]
    fn align_runs_builds_common_grid() {
        let a = run(0.0, 10.0, 5, |d| d * 2.0);
        let b = run(25.0, 5.0, 6, |d| d * 2.0); // 25..50
        let aligned = align_runs(&[&a, &b], Some(5.0));
        assert!(aligned.sample_count() >= 5);
        // First run starts at 0, second at 25.
        assert_eq!(aligned.run_start, vec![0.0, 25.0]);
    }

    #[test]
    fn cross_correlate_finds_known_shift() {
        // Deterministic, non-smooth (hash-based) curve so that only the true
        // alignment correlates; smooth curves can be affinely related under a
        // shift and would produce spurious peaks.
        let curve = |d: f64| -> f64 {
            let h = (d * 12.9898).sin() * 43758.5453;
            (h - h.floor()).abs() * 100.0
        };
        // A single shared formation signal; `shifted` is the same signal
        // recorded 10 m too deep (value at depth x = formation(x - 10)).
        let base = run(100.0, 1.0, 50, curve); // 100..150
        let shifted = run(110.0, 1.0, 50, |d| curve(d - 10.0)); // 110..160 (+10 deeper)
                                                                // `b` is 10 m deeper, so it must be shifted by -10 to match `a`.
        let est = cross_correlate(&base, &shifted, 1, 20.0, 1.0);
        assert!((est - (-10.0)).abs() < 1.5, "estimated shift {est}");
    }
}
