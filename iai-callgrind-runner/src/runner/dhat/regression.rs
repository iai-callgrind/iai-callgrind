use crate::api::{self, DhatMetric};
use crate::runner::metrics::{MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct DhatRegressionConfig {
    pub limits: Vec<(DhatMetric, f64)>,
    pub fail_fast: bool,
}

impl RegressionConfig<DhatMetric> for DhatRegressionConfig {
    fn check(&self, metrics_summary: &MetricsSummary<DhatMetric>) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|(metric, new, old, diff_pct, limit)| ToolRegression {
                metric: MetricKind::Dhat(metric),
                new,
                old,
                diff_pct,
                limit,
            })
            .collect()
    }

    fn get_limits(&self) -> &[(DhatMetric, f64)] {
        &self.limits
    }
}

impl From<api::DhatRegressionConfig> for DhatRegressionConfig {
    fn from(value: api::DhatRegressionConfig) -> Self {
        let api::DhatRegressionConfig { limits, fail_fast } = value;
        DhatRegressionConfig {
            limits: if limits.is_empty() {
                vec![(DhatMetric::TotalBytes, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for DhatRegressionConfig {
    fn default() -> Self {
        Self {
            limits: vec![(DhatMetric::TotalBytes, 10f64)],
            fail_fast: Default::default(),
        }
    }
}
