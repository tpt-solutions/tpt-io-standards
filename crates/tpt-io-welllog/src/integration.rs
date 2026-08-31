//! Feature-gated integration stubs for downstream engineering crates.
//!
//! `tpt-eng-geotech` is not yet published (per `todo.md`). This module provides
//! only the *conversion boundary* — a stable API surface plus a local
//! placeholder type — so the `eng-geotech-integration` feature flag compiles
//! and links today. Once the real crate exists, only the function bodies need
//! to change.

#[cfg(feature = "eng-geotech-integration")]
pub mod eng_geotech {
    //! Stub conversion into a future `tpt_eng_geotech` geotechnical model.

    use crate::align::AlignedLogs;
    use crate::model::WellLog;

    /// Placeholder for a future `tpt_eng_geotech::BoreholeLog`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct BoreholeLogStub {
        /// Number of depth samples.
        pub samples: usize,
        /// Number of curves/channels.
        pub channels: usize,
        /// Start depth (m).
        pub top: f64,
        /// End depth (m).
        pub bottom: f64,
    }

    /// Convert a parsed LAS [`WellLog`] into a borehole-log placeholder.
    pub fn to_borehole_log(log: &WellLog) -> BoreholeLogStub {
        let (top, bottom) = if log.is_empty() {
            (0.0, 0.0)
        } else {
            let a = log.data[0][0];
            let b = log.data[log.data.len() - 1][0];
            (a.min(b), a.max(b))
        };
        BoreholeLogStub { samples: log.len(), channels: log.curves.len(), top, bottom }
    }

    /// Convert an [`AlignedLogs`] correlation result into a multi-run stub.
    pub fn to_multi_run(aligned: &AlignedLogs) -> BoreholeLogStub {
        let top = aligned.depths.first().copied().unwrap_or(0.0);
        let bottom = aligned.depths.last().copied().unwrap_or(0.0);
        BoreholeLogStub {
            samples: aligned.sample_count(),
            channels: aligned.run_count(),
            top: top.min(bottom),
            bottom: top.max(bottom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::eng_geotech::{to_borehole_log, BoreholeLogStub};
    use crate::model::WellLog;

    #[test]
    fn stub_summarizes_log() {
        let log = WellLog {
            version: 2.0,
            wrap: false,
            well: Default::default(),
            parameters: Vec::new(),
            curves: Vec::new(),
            data: vec![vec![100.0, 1.0], vec![200.0, 2.0]],
        };
        let stub: BoreholeLogStub = to_borehole_log(&log);
        assert_eq!(stub.samples, 2);
        assert_eq!(stub.top, 100.0);
        assert_eq!(stub.bottom, 200.0);
    }
}
