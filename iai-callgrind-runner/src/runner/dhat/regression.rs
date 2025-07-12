use crate::api::{self, DhatMetric};
use crate::runner::metrics::{Metric, MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct DhatRegressionConfig {
    pub soft_limits: Vec<(DhatMetric, f64)>,
    pub hard_limits: Vec<(DhatMetric, Metric)>,
    pub fail_fast: bool,
}

impl RegressionConfig<DhatMetric> for DhatRegressionConfig {
    fn check(&self, metrics_summary: &MetricsSummary<DhatMetric>) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|r| ToolRegression::with(MetricKind::Dhat, r))
            .collect()
    }

    fn get_soft_limits(&self) -> &[(DhatMetric, f64)] {
        &self.soft_limits
    }

    fn get_hard_limits(&self) -> &[(DhatMetric, Metric)] {
        &self.hard_limits
    }
}

impl From<api::DhatRegressionConfig> for DhatRegressionConfig {
    fn from(value: api::DhatRegressionConfig) -> Self {
        let api::DhatRegressionConfig { limits, fail_fast } = value;
        DhatRegressionConfig {
            soft_limits: if limits.is_empty() {
                vec![(DhatMetric::TotalBytes, 10f64)]
            } else {
                limits
            },
            hard_limits: Vec::default(),
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for DhatRegressionConfig {
    fn default() -> Self {
        Self {
            soft_limits: vec![(DhatMetric::TotalBytes, 10f64)],
            hard_limits: Vec::default(),
            fail_fast: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use DhatMetric::*;

    use super::*;
    use crate::runner::metrics::Metrics;
    use crate::runner::tool::regression::RegressionMetrics;
    use crate::util::EitherOrBoth;

    fn costs_fixture(costs: [u64; 2]) -> Metrics<DhatMetric> {
        Metrics::with_metric_kinds([(TotalBytes, costs[0]), (TotalBlocks, costs[1])])
    }

    #[rstest]
    #[case::all_zero_no_regression(
        vec![(TotalBytes, 0)],
        [0, 0],
        vec![]
    )]
    #[case::total_bytes_regression_by_one(
        vec![(TotalBytes, 0)],
        [1, 0],
        vec![(TotalBytes, 1, 1, 0)]
    )]
    #[case::total_bytes_regression_by_two(
        vec![(TotalBytes, 0)],
        [2, 0],
        vec![(TotalBytes, 2, 2, 0)]
    )]
    #[case::total_bytes_regression_some_value(
        vec![(TotalBytes, 10)],
        [11, 0],
        vec![(TotalBytes, 11, 1, 10)]
    )]
    #[case::total_bytes_and_block_regression(
        vec![(TotalBytes, 9), (TotalBlocks, 1)],
        [10, 4],
        vec![(TotalBytes, 10, 1, 9), (TotalBlocks, 4, 3, 1)]
    )]
    fn test_regression_check_when_hard<U>(
        #[case] limits: Vec<(DhatMetric, U)>,
        #[case] new: [u64; 2],
        #[case] expected: Vec<(DhatMetric, u64, u64, u64)>,
    ) where
        U: Into<Metric>,
    {
        let regression = DhatRegressionConfig {
            hard_limits: limits.into_iter().map(|(x, y)| (x, y.into())).collect(),
            soft_limits: vec![],
            ..Default::default()
        };

        let new_costs = costs_fixture(new);

        let summary = MetricsSummary::new(EitherOrBoth::Left(new_costs));
        let expected = expected
            .iter()
            .map(|(e, n, d, l)| ToolRegression::Hard {
                metric: MetricKind::Dhat(*e),
                new: (*n).into(),
                diff: (*d).into(),
                limit: (*l).into(),
            })
            .collect::<Vec<ToolRegression>>();

        assert_eq!(regression.check(&summary), expected);
    }

    #[test]
    fn test_regression_check_when_hard_and_soft() {
        let config = DhatRegressionConfig {
            hard_limits: vec![(DhatMetric::TotalBytes, 2.into())],
            soft_limits: vec![(DhatMetric::TotalBlocks, 20f64)],
            ..Default::default()
        };

        let new_costs = costs_fixture([3, 4]);
        let old_costs = costs_fixture([1, 2]);

        let summary = MetricsSummary::new(EitherOrBoth::Both(new_costs, old_costs));
        let expected = vec![
            ToolRegression::with(
                MetricKind::Dhat,
                RegressionMetrics::Soft(DhatMetric::TotalBlocks, 4.into(), 2.into(), 100f64, 20f64),
            ),
            ToolRegression::with(
                MetricKind::Dhat,
                RegressionMetrics::Hard(DhatMetric::TotalBytes, 3.into(), 1.into(), 2.into()),
            ),
        ];

        assert_eq!(config.check(&summary), expected);
    }
}
