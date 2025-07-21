//! The module containing the basic elements for regression check configurations
use std::fmt::Display;
use std::hash::Hash;

use crate::api;
use crate::runner::cachegrind::regression::CachegrindRegressionConfig;
use crate::runner::callgrind::regression::CallgrindRegressionConfig;
use crate::runner::dhat::regression::DhatRegressionConfig;
use crate::runner::format::print_regressions;
use crate::runner::metrics::{Metric, MetricsSummary, Summarize};
use crate::runner::summary::ToolRegression;
use crate::util::EitherOrBoth;

/// A short-lived utility enum used to hold the raw regressions until they can be transformed into a
/// real [`ToolRegression`]
pub enum RegressionMetrics<T> {
    /// The result of a checked soft limit
    Soft(T, Metric, Metric, f64, f64),
    /// The result of a checked hard limit
    Hard(T, Metric, Metric, Metric),
}

/// The tool specific regression check configuration
#[derive(Debug, Clone, PartialEq)]
pub enum ToolRegressionConfig {
    /// The Callgrind configuration
    Callgrind(CallgrindRegressionConfig),
    /// The Cachegrind configuration
    Cachegrind(CachegrindRegressionConfig),
    /// The DHAT configuration
    Dhat(DhatRegressionConfig),
    /// If there is no configuration
    None,
}

/// The trait which needs to be implemented in a tool specific regression check configuration
pub trait RegressionConfig<T: Hash + Eq + Summarize + Display + Clone> {
    /// Check for regressions and print them if present
    fn check_and_print(&self, metrics_summary: &MetricsSummary<T>) -> Vec<ToolRegression> {
        let regressions = self.check(metrics_summary);
        print_regressions(&regressions);
        regressions
    }

    /// Check the `MetricsSummary` for regressions.
    ///
    /// The limits for event kinds which are not present in the `MetricsSummary` are ignored.
    fn check(&self, metrics_summary: &MetricsSummary<T>) -> Vec<ToolRegression>;
    /// Check for regressions and return the [`RegressionMetrics`]
    fn check_regressions(&self, metrics_summary: &MetricsSummary<T>) -> Vec<RegressionMetrics<T>> {
        let mut regressions = vec![];
        for (metric, new_cost, old_cost, pct, limit) in
            self.get_soft_limits().iter().filter_map(|(kind, limit)| {
                metrics_summary.diff_by_kind(kind).and_then(|d| {
                    if let EitherOrBoth::Both(new, old) = d.metrics {
                        // This unwrap is safe since the diffs are calculated if both costs are
                        // present
                        Some((kind, new, old, d.diffs.unwrap().diff_pct, limit))
                    } else {
                        None
                    }
                })
            })
        {
            if limit.is_sign_positive() {
                if pct > *limit {
                    regressions.push(RegressionMetrics::Soft(
                        metric.clone(),
                        new_cost,
                        old_cost,
                        pct,
                        *limit,
                    ));
                }
            } else if pct < *limit {
                regressions.push(RegressionMetrics::Soft(
                    metric.clone(),
                    new_cost,
                    old_cost,
                    pct,
                    *limit,
                ));
            } else {
                // no regression
            }
        }

        for (metric, new_cost, limit) in
            self.get_hard_limits().iter().filter_map(|(kind, limit)| {
                metrics_summary
                    .diff_by_kind(kind)
                    .and_then(|d| d.metrics.left().map(|metric| (kind, metric, limit)))
            })
        {
            if new_cost > limit {
                regressions.push(RegressionMetrics::Hard(
                    metric.clone(),
                    *new_cost,
                    *new_cost - *limit,
                    *limit,
                ));
            }
        }
        regressions
    }

    /// Return the soft limits
    fn get_soft_limits(&self) -> &[(T, f64)];
    /// Return the hard limits
    fn get_hard_limits(&self) -> &[(T, Metric)];
}

impl ToolRegressionConfig {
    /// Return true if the configuration has fail fast set to true
    pub fn is_fail_fast(&self) -> bool {
        match self {
            ToolRegressionConfig::Callgrind(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::Cachegrind(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::Dhat(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::None => false,
        }
    }
}

impl TryFrom<api::ToolRegressionConfig> for ToolRegressionConfig {
    type Error = String;

    fn try_from(value: api::ToolRegressionConfig) -> std::result::Result<Self, Self::Error> {
        match value {
            api::ToolRegressionConfig::Callgrind(regression_config) => {
                regression_config.try_into().map(Self::Callgrind)
            }
            api::ToolRegressionConfig::Cachegrind(regression_config) => {
                regression_config.try_into().map(Self::Cachegrind)
            }
            api::ToolRegressionConfig::Dhat(regression_config) => {
                regression_config.try_into().map(Self::Dhat)
            }
            api::ToolRegressionConfig::None => Ok(Self::None),
        }
    }
}
