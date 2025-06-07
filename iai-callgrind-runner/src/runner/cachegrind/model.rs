use std::borrow::Cow;

use anyhow::Result;
use indexmap::indexmap;

use crate::api::CachegrindMetric;
use crate::runner::callgrind::{CacheSummary, CyclesEstimator};
use crate::runner::metrics::Summarize;

pub type Metrics = crate::runner::metrics::Metrics<CachegrindMetric>;

impl Default for Metrics {
    fn default() -> Self {
        Self(indexmap! {CachegrindMetric::Ir => 0})
    }
}

impl Summarize for CachegrindMetric {
    fn summarize(costs: &mut Cow<Metrics>) {
        if !costs.is_summarized() {
            let _ = costs.to_mut().make_summary();
        }
    }
}

impl TryFrom<&Metrics> for CacheSummary {
    type Error = anyhow::Error;

    fn try_from(value: &Metrics) -> std::result::Result<Self, Self::Error> {
        use CachegrindMetric::*;
        let estimator = CyclesEstimator::new(
            value.try_metric_by_kind(&Ir)?,
            value.try_metric_by_kind(&Dr)?,
            value.try_metric_by_kind(&Dw)?,
            value.try_metric_by_kind(&I1mr)?,
            value.try_metric_by_kind(&D1mr)?,
            value.try_metric_by_kind(&D1mw)?,
            value.try_metric_by_kind(&ILmr)?,
            value.try_metric_by_kind(&DLmr)?,
            value.try_metric_by_kind(&DLmw)?,
        );

        Ok(estimator.calculate())
    }
}

/// TODO: UPDATE DOCS from callgrind to cachegrind, event kinds to cachegrind metrics,...
/// TODO: This is very similar to the impl in `callgrind::model`
impl Metrics {
    /// Calculate and add derived summary events (i.e. estimated cycles) in-place
    ///
    /// Additional calls to this function will overwrite the metrics for derived summary events.
    ///
    /// # Errors
    ///
    /// If the necessary cache simulation events (when running callgrind with --cache-sim) were not
    /// present.
    pub fn make_summary(&mut self) -> Result<()> {
        let CacheSummary {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        } = (&*self).try_into()?;

        self.insert(CachegrindMetric::L1hits, l1_hits);
        self.insert(CachegrindMetric::LLhits, l3_hits);
        self.insert(CachegrindMetric::RamHits, ram_hits);
        self.insert(CachegrindMetric::TotalRW, total_memory_rw);
        self.insert(CachegrindMetric::EstimatedCycles, cycles);

        Ok(())
    }

    /// Return true if costs are already summarized
    ///
    /// This method just probes for [`EventKind::EstimatedCycles`] to detect the summarized state.
    pub fn is_summarized(&self) -> bool {
        self.metric_by_kind(&CachegrindMetric::EstimatedCycles)
            .is_some()
    }

    /// Return true if costs can be summarized
    ///
    /// This method probes for [`EventKind::I1mr`] which is present if callgrind was run with the
    /// cache simulation (`--cache-sim=yes`) enabled.
    pub fn can_summarize(&self) -> bool {
        self.metric_by_kind(&CachegrindMetric::I1mr).is_some()
    }
}
