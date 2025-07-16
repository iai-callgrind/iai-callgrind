use std::fmt::Display;
use std::hash::Hash;

use crate::runner::format::print_regressions;
use crate::runner::metrics::{Metric, MetricsSummary, Summarize};
use crate::runner::summary::ToolRegression;
use crate::util::EitherOrBoth;

/// A short-lived utility enum used to hold the raw regressions until they can be transformed into a
/// real [`ToolRegression`]
pub enum RegressionMetrics<T> {
    Soft(T, Metric, Metric, f64, f64),
    Hard(T, Metric, Metric, Metric),
}

pub trait RegressionConfig<T: Hash + Eq + Summarize + Display + Clone> {
    fn check_and_print(&self, metrics_summary: &MetricsSummary<T>) -> Vec<ToolRegression> {
        let regressions = self.check(metrics_summary);
        print_regressions(&regressions);
        regressions
    }

    // Check the `MetricsSummary` for regressions.
    //
    // The limits for event kinds which are not present in the `MetricsSummary` are ignored.
    fn check(&self, metrics_summary: &MetricsSummary<T>) -> Vec<ToolRegression>;
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

    fn get_soft_limits(&self) -> &[(T, f64)];
    fn get_hard_limits(&self) -> &[(T, Metric)];
}
