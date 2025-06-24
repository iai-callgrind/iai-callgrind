use std::borrow::Cow;

use anyhow::Result;
use indexmap::indexmap;

use crate::api::CachegrindMetric;
use crate::runner::callgrind::{CacheSummary, CyclesEstimator};
use crate::runner::metrics::{Metric, Summarize};

pub type Metrics = crate::runner::metrics::Metrics<CachegrindMetric>;

impl Default for Metrics {
    fn default() -> Self {
        Self(indexmap! {CachegrindMetric::Ir => Metric::Int(0)})
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

impl Metrics {
    /// Calculate and add derived summary events (i.e. estimated cycles) in-place
    ///
    /// Additional calls to this function will overwrite the metrics for derived summary events.
    ///
    /// # Errors
    ///
    /// If the necessary cache simulation events (when running cachegrind with --cache-sim) were not
    /// present.
    pub fn make_summary(&mut self) -> Result<()> {
        let CacheSummary {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
            i1_miss_rate,
            d1_miss_rate,
            ll_miss_rate,
            lli_miss_rate,
            lld_miss_rate,
            l1_hit_rate,
            l3_hit_rate,
            ram_hit_rate,
        } = (&*self).try_into()?;

        self.insert(CachegrindMetric::L1hits, l1_hits);
        self.insert(CachegrindMetric::LLhits, l3_hits);
        self.insert(CachegrindMetric::RamHits, ram_hits);
        self.insert(CachegrindMetric::TotalRW, total_memory_rw);
        self.insert(CachegrindMetric::EstimatedCycles, cycles);
        self.insert(CachegrindMetric::I1MissRate, i1_miss_rate);
        self.insert(CachegrindMetric::D1MissRate, d1_miss_rate);
        self.insert(CachegrindMetric::LLiMissRate, lli_miss_rate);
        self.insert(CachegrindMetric::LLdMissRate, lld_miss_rate);
        self.insert(CachegrindMetric::LLMissRate, ll_miss_rate);
        self.insert(CachegrindMetric::L1HitRate, l1_hit_rate);
        self.insert(CachegrindMetric::LLHitRate, l3_hit_rate);
        self.insert(CachegrindMetric::RamHitRate, ram_hit_rate);

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
    /// This method probes for [`EventKind::I1mr`] which is present if cachegrind was run with the
    /// cache simulation (`--cache-sim=yes`) enabled.
    pub fn can_summarize(&self) -> bool {
        self.metric_by_kind(&CachegrindMetric::I1mr).is_some()
    }
}
