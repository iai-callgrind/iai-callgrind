use crate::api::{self, CachegrindMetric};
use crate::runner::metrics::{MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct CachegrindRegressionConfig {
    pub limits: Vec<(CachegrindMetric, f64)>,
    pub fail_fast: bool,
}

impl RegressionConfig<CachegrindMetric> for CachegrindRegressionConfig {
    fn check(&self, metrics_summary: &MetricsSummary<CachegrindMetric>) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|r| ToolRegression::with(MetricKind::Cachegrind, r))
            .collect()
    }

    fn get_soft_limits(&self) -> &[(CachegrindMetric, f64)] {
        &self.limits
    }

    fn get_hard_limits(&self) -> &[(CachegrindMetric, crate::runner::metrics::Metric)] {
        // TODO: Hard limits for Cachegrind
        &[]
    }
}

impl From<api::CachegrindRegressionConfig> for CachegrindRegressionConfig {
    fn from(value: api::CachegrindRegressionConfig) -> Self {
        let api::CachegrindRegressionConfig { limits, fail_fast } = value;
        CachegrindRegressionConfig {
            limits: if limits.is_empty() {
                vec![(CachegrindMetric::Ir, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for CachegrindRegressionConfig {
    fn default() -> Self {
        Self {
            limits: vec![(CachegrindMetric::Ir, 10f64)],
            fail_fast: Default::default(),
        }
    }
}
