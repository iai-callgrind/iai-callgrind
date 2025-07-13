use crate::api::{self, CachegrindMetric};
use crate::runner::metrics::{Metric, MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct CachegrindRegressionConfig {
    pub soft_limits: Vec<(CachegrindMetric, f64)>,
    pub hard_limits: Vec<(CachegrindMetric, Metric)>,
    pub fail_fast: bool,
}

impl RegressionConfig<CachegrindMetric> for CachegrindRegressionConfig {
    fn check(&self, metrics_summary: &MetricsSummary<CachegrindMetric>) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|regressions| ToolRegression::with(MetricKind::Cachegrind, regressions))
            .collect()
    }

    fn get_soft_limits(&self) -> &[(CachegrindMetric, f64)] {
        &self.soft_limits
    }

    fn get_hard_limits(&self) -> &[(CachegrindMetric, Metric)] {
        &self.hard_limits
    }
}

impl From<api::CachegrindRegressionConfig> for CachegrindRegressionConfig {
    fn from(value: api::CachegrindRegressionConfig) -> Self {
        let api::CachegrindRegressionConfig {
            soft_limits,
            hard_limits,
            fail_fast,
        } = value;
        CachegrindRegressionConfig {
            soft_limits: if soft_limits.is_empty() && hard_limits.is_empty() {
                vec![(CachegrindMetric::Ir, 10f64)]
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

impl Default for CachegrindRegressionConfig {
    fn default() -> Self {
        Self {
            soft_limits: vec![(CachegrindMetric::Ir, 10f64)],
            hard_limits: Vec::default(),
            fail_fast: false,
        }
    }
}
