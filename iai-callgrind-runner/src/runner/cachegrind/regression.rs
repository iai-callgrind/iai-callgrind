//! Module containing the cachegrind specific regression check configuration
use indexmap::{IndexMap, IndexSet};

use crate::api::{self, CachegrindMetric};
use crate::runner::metrics::{Metric, MetricKind, MetricsSummary};
use crate::runner::summary::ToolRegression;
use crate::runner::tool::regression::RegressionConfig;

/// The callgrind regression check configuration
#[derive(Debug, Clone, PartialEq)]
pub struct CachegrindRegressionConfig {
    /// True if benchmarks should fail on first encountered failed regression check
    pub fail_fast: bool,
    /// The hard limits
    pub hard_limits: Vec<(CachegrindMetric, Metric)>,
    /// The soft limits
    pub soft_limits: Vec<(CachegrindMetric, f64)>,
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

impl TryFrom<api::CachegrindRegressionConfig> for CachegrindRegressionConfig {
    type Error = String;

    fn try_from(value: api::CachegrindRegressionConfig) -> Result<Self, Self::Error> {
        let api::CachegrindRegressionConfig {
            soft_limits,
            hard_limits,
            fail_fast,
        } = value;

        let (soft_limits, hard_limits) = if soft_limits.is_empty() && hard_limits.is_empty() {
            (
                IndexMap::from([(CachegrindMetric::Ir, 10f64)]),
                IndexMap::new(),
            )
        } else {
            let hard_limits = hard_limits
                .into_iter()
                .flat_map(|(cachegrind_metrics, metric)| {
                    IndexSet::from(cachegrind_metrics)
                        .into_iter()
                        .map(move |metric_kind| {
                            Metric::from(metric)
                                .try_convert(metric_kind)
                                .ok_or_else(|| {
                                    format!(
                                        "Invalid hard limit for \
                                         '{metric_kind:?}/{cachegrind_metrics:?}': Expected a \
                                         'Int' but found '{metric:?}'"
                                    )
                                })
                        })
                })
                .collect::<Result<IndexMap<CachegrindMetric, Metric>, String>>()?;

            let soft_limits = soft_limits
                .into_iter()
                .flat_map(|(m, l)| IndexSet::from(m).into_iter().map(move |e| (e, l)))
                .collect::<IndexMap<_, _>>();

            (soft_limits, hard_limits)
        };

        Ok(Self {
            soft_limits: soft_limits.into_iter().collect(),
            hard_limits: hard_limits.into_iter().collect(),
            fail_fast: fail_fast.unwrap_or(false),
        })
    }
}
