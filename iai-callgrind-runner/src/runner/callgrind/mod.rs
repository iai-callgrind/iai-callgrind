pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod summary_parser;

use std::path::PathBuf;

use parser::CallgrindProperties;

use self::model::Metrics;
use super::summary::{MetricKind, MetricsSummary, ToolRegression};
use super::tool::RegressionConfig;
use crate::api::{self, EventKind};
use crate::util::EitherOrBoth;

#[derive(Debug, Clone)]
pub struct Summary {
    pub details: EitherOrBoth<(PathBuf, CallgrindProperties)>,
    pub metrics_summary: MetricsSummary,
}

#[derive(Debug, Clone)]
pub struct Summaries {
    pub summaries: Vec<Summary>,
    pub total: MetricsSummary,
}

#[derive(Clone, Debug)]
pub struct CacheSummary {
    pub l1_hits: u64,
    pub l3_hits: u64,
    pub ram_hits: u64,
    pub total_memory_rw: u64,
    pub cycles: u64,
}

#[derive(Debug, Clone)]
pub struct CyclesEstimator {
    instructions: u64,
    total_data_cache_reads: u64,
    total_data_cache_writes: u64,
    l1_instructions_cache_read_misses: u64,
    l1_data_cache_read_misses: u64,
    l1_data_cache_write_misses: u64,
    l3_instructions_cache_read_misses: u64,
    l3_data_cache_read_misses: u64,
    l3_data_cache_write_misses: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallgrindRegressionConfig {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: bool,
}

impl TryFrom<&Metrics> for CacheSummary {
    type Error = anyhow::Error;

    fn try_from(value: &Metrics) -> std::result::Result<Self, Self::Error> {
        use EventKind::*;
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

impl CyclesEstimator {
    pub fn new(
        instructions: u64,
        total_data_cache_reads: u64,
        total_data_cache_writes: u64,
        l1_instructions_cache_read_misses: u64,
        l1_data_cache_read_misses: u64,
        l1_data_cache_write_misses: u64,
        l3_instructions_cache_read_misses: u64,
        l3_data_cache_read_misses: u64,
        l3_data_cache_write_misses: u64,
    ) -> Self {
        Self {
            instructions,
            total_data_cache_reads,
            total_data_cache_writes,
            l1_instructions_cache_read_misses,
            l1_data_cache_read_misses,
            l1_data_cache_write_misses,
            l3_instructions_cache_read_misses,
            l3_data_cache_read_misses,
            l3_data_cache_write_misses,
        }
    }

    pub fn calculate(&self) -> CacheSummary {
        let ram_hits = self.l3_instructions_cache_read_misses
            + self.l3_data_cache_read_misses
            + self.l3_data_cache_write_misses;
        let l1_data_accesses = self.l1_data_cache_read_misses + self.l1_data_cache_write_misses;
        let l1_miss = self.l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw =
            self.instructions + self.total_data_cache_reads + self.total_data_cache_writes;
        let l1_hits = total_memory_rw - ram_hits - l3_hits;

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles = l1_hits + (5 * l3_hits) + (35 * ram_hits);

        CacheSummary {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        }
    }
}

impl RegressionConfig<EventKind> for CallgrindRegressionConfig {
    // Check the `MetricsSummary` for regressions.
    //
    // The limits for event kinds which are not present in the `MetricsSummary` are ignored.
    fn check(&self, metrics_summary: &MetricsSummary) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|(metric, new, old, diff_pct, limit)| ToolRegression {
                metric: MetricKind::Callgrind(metric),
                new,
                old,
                diff_pct,
                limit,
            })
            .collect()
    }

    fn get_limits(&self) -> &[(EventKind, f64)] {
        &self.limits
    }
}

impl From<api::CallgrindRegressionConfig> for CallgrindRegressionConfig {
    fn from(value: api::CallgrindRegressionConfig) -> Self {
        let api::CallgrindRegressionConfig { limits, fail_fast } = value;
        CallgrindRegressionConfig {
            limits: if limits.is_empty() {
                vec![(EventKind::Ir, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for CallgrindRegressionConfig {
    fn default() -> Self {
        Self {
            limits: vec![(EventKind::Ir, 10f64)],
            fail_fast: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use EventKind::*;

    use super::*;
    use crate::runner::summary::MetricKind;

    fn cachesim_costs(costs: [u64; 9]) -> Metrics {
        Metrics::with_metric_kinds([
            (Ir, costs[0]),
            (Dr, costs[1]),
            (Dw, costs[2]),
            (I1mr, costs[3]),
            (D1mr, costs[4]),
            (D1mw, costs[5]),
            (ILmr, costs[6]),
            (DLmr, costs[7]),
            (DLmw, costs[8]),
        ])
    }

    #[rstest]
    fn test_regression_check_when_old_is_none() {
        let regression = CallgrindRegressionConfig::default();
        let new = cachesim_costs([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let summary = MetricsSummary::new(EitherOrBoth::Left(new));

        assert!(regression.check(&summary).is_empty());
    }

    #[rstest]
    #[case::ir_all_zero(
        vec![(Ir, 0f64)],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::ir_when_regression(
        vec![(Ir, 0f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(Ir, 2, 1, 100f64, 0f64)]
    )]
    #[case::ir_when_improved(
        vec![(Ir, 0f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::ir_when_negative_limit(
        vec![(Ir, -49f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(Ir, 1, 2, -50f64, -49f64)]
    )]
    #[case::derived_all_zero(
        vec![(EstimatedCycles, 0f64)],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::derived_when_regression(
        vec![(EstimatedCycles, 0f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(EstimatedCycles, 2, 1, 100f64, 0f64)]
    )]
    #[case::derived_when_regression_multiple(
        vec![(EstimatedCycles, 5f64), (Ir, 10f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(EstimatedCycles, 2, 1, 100f64, 5f64), (Ir, 2, 1, 100f64, 10f64)]
    )]
    #[case::derived_when_improved(
        vec![(EstimatedCycles, 0f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::derived_when_regression_mixed(
        vec![(EstimatedCycles, 0f64)],
        [96, 24, 18, 6, 0, 2, 6, 0, 2],
        [48, 12, 9, 3, 0, 1, 3, 0, 1],
        vec![(EstimatedCycles, 410, 205, 100f64, 0f64)]
    )]
    fn test_regression_check_when_old_is_some(
        #[case] limits: Vec<(EventKind, f64)>,
        #[case] new: [u64; 9],
        #[case] old: [u64; 9],
        #[case] expected: Vec<(EventKind, u64, u64, f64, f64)>,
    ) {
        let regression = CallgrindRegressionConfig {
            limits,
            ..Default::default()
        };

        let new = cachesim_costs(new);
        let old = cachesim_costs(old);
        let summary = MetricsSummary::new(EitherOrBoth::Both(new, old));
        let expected = expected
            .iter()
            .map(|(e, n, o, d, l)| ToolRegression {
                metric: MetricKind::Callgrind(*e),
                new: *n,
                old: *o,
                diff_pct: *d,
                limit: *l,
            })
            .collect::<Vec<ToolRegression>>();

        assert_eq!(regression.check(&summary), expected);
    }
}
