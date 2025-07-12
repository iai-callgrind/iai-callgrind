use crate::api::{self, EventKind};
use crate::runner::metrics::{Metric, MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct CallgrindRegressionConfig {
    pub soft_limits: Vec<(EventKind, f64)>,
    pub hard_limits: Vec<(EventKind, Metric)>,
    pub fail_fast: bool,
}

impl RegressionConfig<EventKind> for CallgrindRegressionConfig {
    /// Check the `MetricsSummary` for regressions.
    ///
    /// The limits for event kinds which are not present in the `MetricsSummary` are ignored.
    fn check(&self, metrics_summary: &MetricsSummary) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|regressions| ToolRegression::with(MetricKind::Callgrind, regressions))
            .collect()
    }

    fn get_soft_limits(&self) -> &[(EventKind, f64)] {
        &self.soft_limits
    }

    fn get_hard_limits(&self) -> &[(EventKind, Metric)] {
        &self.hard_limits
    }
}

impl From<api::CallgrindRegressionConfig> for CallgrindRegressionConfig {
    fn from(value: api::CallgrindRegressionConfig) -> Self {
        let api::CallgrindRegressionConfig {
            soft_limits,
            hard_limits,
            fail_fast,
        } = value;
        CallgrindRegressionConfig {
            soft_limits: if soft_limits.is_empty() && hard_limits.is_empty() {
                vec![(EventKind::Ir, 10f64)]
            } else {
                soft_limits
            },
            hard_limits: hard_limits
                .into_iter()
                .map(|(m, l)| (m, l.into()))
                .collect(),
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for CallgrindRegressionConfig {
    fn default() -> Self {
        Self {
            soft_limits: vec![(EventKind::Ir, 10f64)],
            hard_limits: Vec::default(),
            fail_fast: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use EventKind::*;

    use super::*;
    use crate::runner::callgrind::model::Metrics;
    use crate::runner::metrics::Metric;
    use crate::util::EitherOrBoth;

    fn cachesim_costs(costs: [u64; 9]) -> Metrics {
        Metrics::with_metric_kinds([
            (Ir, Metric::Int(costs[0])),
            (Dr, Metric::Int(costs[1])),
            (Dw, Metric::Int(costs[2])),
            (I1mr, Metric::Int(costs[3])),
            (D1mr, Metric::Int(costs[4])),
            (D1mw, Metric::Int(costs[5])),
            (ILmr, Metric::Int(costs[6])),
            (DLmr, Metric::Int(costs[7])),
            (DLmw, Metric::Int(costs[8])),
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
    fn test_regression_check_when_soft_and_old_is_some(
        #[case] soft_limits: Vec<(EventKind, f64)>,
        #[case] new: [u64; 9],
        #[case] old: [u64; 9],
        #[case] expected: Vec<(EventKind, u64, u64, f64, f64)>,
    ) {
        let regression = CallgrindRegressionConfig {
            soft_limits,
            ..Default::default()
        };

        let new = cachesim_costs(new);
        let old = cachesim_costs(old);
        let summary = MetricsSummary::new(EitherOrBoth::Both(new, old));
        let expected = expected
            .iter()
            .map(|(e, n, o, d, l)| ToolRegression::Soft {
                metric: MetricKind::Callgrind(*e),
                new: (*n).into(),
                old: (*o).into(),
                diff_pct: *d,
                limit: *l,
            })
            .collect::<Vec<ToolRegression>>();

        assert_eq!(regression.check(&summary), expected);
    }
}
