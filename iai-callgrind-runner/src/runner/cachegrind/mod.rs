use super::summary::{MetricKind, MetricsSummary, ToolRegression};
use super::tool::RegressionConfig;
use crate::api::{self, CachegrindMetric};

pub mod args;
pub mod model;
pub mod parser;
pub mod summary_parser;

#[derive(Debug, Clone, PartialEq)]
pub struct CachegrindRegressionConfig {
    pub limits: Vec<(CachegrindMetric, f64)>,
    pub fail_fast: bool,
}

impl RegressionConfig<CachegrindMetric> for CachegrindRegressionConfig {
    fn check(&self, metrics_summary: &MetricsSummary<CachegrindMetric>) -> Vec<ToolRegression> {
        self.check_regressions(metrics_summary)
            .into_iter()
            .map(|(metric, new, old, diff_pct, limit)| ToolRegression {
                metric: MetricKind::Cachegrind(metric),
                new,
                old,
                diff_pct,
                limit,
            })
            .collect()
    }

    fn get_limits(&self) -> &[(CachegrindMetric, f64)] {
        &self.limits
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
