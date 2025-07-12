use std::fmt::Display;
use std::hash::Hash;

use crate::runner::format::print_regressions;
use crate::runner::metrics::{MetricsSummary, Summarize};
use crate::runner::summary::{RegressionMetrics, ToolRegression};
use crate::util::EitherOrBoth;

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
            self.get_limits().iter().filter_map(|(kind, limit)| {
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
                    regressions.push((metric.clone(), new_cost, old_cost, pct, *limit));
                }
            } else if pct < *limit {
                regressions.push((metric.clone(), new_cost, old_cost, pct, *limit));
            } else {
                // no regression
            }
        }
        regressions
    }

    fn get_limits(&self) -> &[(T, f64)];
}
