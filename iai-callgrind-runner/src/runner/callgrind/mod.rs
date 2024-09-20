pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod summary_parser;

use std::convert::Into;
use std::path::PathBuf;

use colored::Colorize;
use itertools::Itertools;
use parser::{CallgrindProperties, ParserOutput};

use self::model::Metrics;
use super::summary::{
    CallgrindRegression, MetricsSummary, ToolMetricSummary, ToolRun, ToolRunSegment,
};
use crate::api::{self, EventKind};
use crate::util::{to_string_signed_short, EitherOrBoth};

#[derive(Debug, Clone)]
pub struct Summary {
    pub details: EitherOrBoth<(PathBuf, CallgrindProperties)>,
    pub metrics_summary: MetricsSummary,
}

#[derive(Debug, Clone)]
pub struct Summaries {
    pub summaries: Vec<Summary>,
    pub total: MetricsSummary,
}

#[derive(Clone, Debug)]
pub struct CacheSummary {
    l1_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
    total_memory_rw: u64,
    cycles: u64,
}

#[derive(Debug, Clone)]
pub struct RegressionConfig {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: bool,
}

impl TryFrom<&Metrics> for CacheSummary {
    type Error = anyhow::Error;

    fn try_from(value: &Metrics) -> std::result::Result<Self, Self::Error> {
        use EventKind::*;
        //         0   1  2    3    4    5    6    7    8
        // events: Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let instructions = value.try_metric_by_kind(&Ir)?;
        let total_data_cache_reads = value.try_metric_by_kind(&Dr)?;
        let total_data_cache_writes = value.try_metric_by_kind(&Dw)?;
        let l1_instructions_cache_read_misses = value.try_metric_by_kind(&I1mr)?;
        let l1_data_cache_read_misses = value.try_metric_by_kind(&D1mr)?;
        let l1_data_cache_write_misses = value.try_metric_by_kind(&D1mw)?;
        let l3_instructions_cache_read_misses = value.try_metric_by_kind(&ILmr)?;
        let l3_data_cache_read_misses = value.try_metric_by_kind(&DLmr)?;
        let l3_data_cache_write_misses = value.try_metric_by_kind(&DLmw)?;

        let ram_hits = l3_instructions_cache_read_misses
            + l3_data_cache_read_misses
            + l3_data_cache_write_misses;
        let l1_data_accesses = l1_data_cache_read_misses + l1_data_cache_write_misses;
        let l1_miss = l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw = instructions + total_data_cache_reads + total_data_cache_writes;
        let l1_hits = total_memory_rw - ram_hits - l3_hits;

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles = l1_hits + (5 * l3_hits) + (35 * ram_hits);

        Ok(Self {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        })
    }
}

impl RegressionConfig {
    /// Check regression of the [`Costs`] for the configured [`EventKind`]s and print it
    ///
    /// If the old `Costs` is None then no regression checks are performed and this method returns
    /// [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns an [`anyhow::Error`] with the only source [`crate::error::Error::RegressionError`]
    /// if a regression error occurred
    pub fn check_and_print(&self, metrics_summary: &MetricsSummary) -> Vec<CallgrindRegression> {
        let regression = self.check(metrics_summary);

        for CallgrindRegression {
            event_kind,
            new,
            old,
            diff_pct,
            limit,
        } in &regression
        {
            if limit.is_sign_positive() {
                eprintln!(
                    "Performance has {0}: {1} ({new} > {old}) regressed by {2:>+6} (>{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(*diff_pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(*limit).bright_black()
                );
            } else {
                eprintln!(
                    "Performance has {0}: {1} ({new} < {old}) regressed by {2:>+6} (<{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(*diff_pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(*limit).bright_black()
                );
            }
        }

        regression
    }

    // Check the `CostsSummary` for regressions.
    //
    // The limits for event kinds which are not present in the `CostsSummary` are ignored. A
    // `CostsDiff` which does not have both `new` and `old` is also ignored.
    pub fn check(&self, metrics_summary: &MetricsSummary) -> Vec<CallgrindRegression> {
        let mut regressions = vec![];
        for (event_kind, new_cost, old_cost, pct, limit) in
            self.limits.iter().filter_map(|(event_kind, limit)| {
                metrics_summary.diff_by_kind(event_kind).and_then(|d| {
                    if let EitherOrBoth::Both(new, old) = d.metrics {
                        // This unwrap is safe since the diffs are calculated if both costs are
                        // present
                        Some((event_kind, new, old, d.diffs.unwrap().diff_pct, limit))
                    } else {
                        None
                    }
                })
            })
        {
            if limit.is_sign_positive() {
                if pct > *limit {
                    let regression = CallgrindRegression {
                        event_kind: *event_kind,
                        new: new_cost,
                        old: old_cost,
                        diff_pct: pct,
                        limit: *limit,
                    };
                    regressions.push(regression);
                }
            } else if pct < *limit {
                let regression = CallgrindRegression {
                    event_kind: *event_kind,
                    new: new_cost,
                    old: old_cost,
                    diff_pct: pct,
                    limit: *limit,
                };
                regressions.push(regression);
            } else {
                // no regression
            }
        }
        regressions
    }
}

/// TODO: MOVE DEFAULT values into defaults mod
impl From<api::RegressionConfig> for RegressionConfig {
    fn from(value: api::RegressionConfig) -> Self {
        let api::RegressionConfig { limits, fail_fast } = value;
        RegressionConfig {
            limits: if limits.is_empty() {
                vec![(EventKind::Ir, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

/// TODO: MOVE DEFAULT values into defaults mod
impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            limits: vec![(EventKind::Ir, 10f64)],
            fail_fast: Default::default(),
        }
    }
}

impl Summaries {
    /// Group the output by pid, then by parts and then by threads
    ///
    /// The grouping simplifies the zipping of the new and old parser output later.
    ///
    /// A simplified example. `(pid, part, thread)`
    ///
    /// ```rust,ignore
    /// let parsed: Vec<(i32, u64, usize)> = [
    ///     (10, 1, 1),
    ///     (10, 1, 2),
    ///     (20, 1, 1)
    /// ];
    ///
    /// let grouped = group(parsed);
    /// assert_eq!(grouped,
    /// vec![
    ///     vec![
    ///         vec![
    ///             (10, 1, 1),
    ///             (10, 1, 2)
    ///         ]
    ///     ],
    ///     vec![
    ///         vec![
    ///             (20, 1, 1)
    ///         ]
    ///     ]
    /// ])
    /// ```
    fn group(
        parsed: impl Iterator<Item = (PathBuf, CallgrindProperties, Metrics)>,
    ) -> Vec<Vec<Vec<(PathBuf, CallgrindProperties, Metrics)>>> {
        let mut grouped = vec![];
        let mut cur_pid = 0_i32;
        let mut cur_part = 0;

        for element in parsed {
            let pid = element.1.pid.unwrap_or(0_i32);
            let part = element.1.part.unwrap_or(0);

            if pid != cur_pid {
                grouped.push(vec![vec![element]]);
                cur_pid = pid;
                cur_part = part;
            } else if part != cur_part {
                let parts = grouped.last_mut().unwrap();
                parts.push(vec![element]);
                cur_part = part;
            } else {
                let parts = grouped.last_mut().unwrap();
                let threads = parts.last_mut().unwrap();
                threads.push(element);
            }
        }
        grouped
    }

    /// Create a new `Summaries` from the output(s) of the callgrind parser.
    ///
    /// The summaries created from the new parser outputs and the old parser outputs are grouped by
    /// pid (subprocesses recorded with `--trace-children`), then by part (for example cause by a
    /// `--dump-every-bb=xxx`) and then by thread (caused by `--separate-threads`). Since each of
    /// these components can differ between the new and the old parser output, this complicates the
    /// creation of each `Summary`. We can't just zip the new and old parser output directly to get
    /// (as far as possible) correct comparisons between the new and old costs. To remedy the
    /// possibly incorrect comparisons, there is always a total created.
    ///
    /// In a first step the parsed outputs are grouped in vectors by pid, then by parts and then by
    /// threads. This solution is not very efficient but there are not too many parsed outputs to be
    /// expected. 100 at most and maybe 2-10 on average, so the tradeoff between performance and
    /// clearer structure of this method looks reasonable.
    ///
    /// Secondly and finally, the groups are processed and summarized in a total.
    pub fn new(parsed_new: ParserOutput, parsed_old: Option<ParserOutput>) -> Self {
        let grouped_new = Self::group(parsed_new.into_iter());
        let grouped_old = Self::group(parsed_old.into_iter().flatten());

        let mut total = MetricsSummary::default();
        let mut summaries = vec![];

        for e_pids in grouped_new.into_iter().zip_longest(grouped_old) {
            match e_pids {
                itertools::EitherOrBoth::Both(new_parts, old_parts) => {
                    for e_parts in new_parts.into_iter().zip_longest(old_parts) {
                        match e_parts {
                            itertools::EitherOrBoth::Both(new_threads, old_threads) => {
                                for e_threads in new_threads.into_iter().zip_longest(old_threads) {
                                    let summary = match e_threads {
                                        itertools::EitherOrBoth::Both(new, old) => {
                                            Summary::from_new_and_old(new, old)
                                        }
                                        itertools::EitherOrBoth::Left(new) => {
                                            Summary::from_new(new.0, new.1, new.2)
                                        }
                                        itertools::EitherOrBoth::Right(old) => {
                                            Summary::from_old(old.0, old.1, old.2)
                                        }
                                    };
                                    total.add(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                            itertools::EitherOrBoth::Left(left) => {
                                for new in left {
                                    let summary = Summary::from_new(new.0, new.1, new.2);
                                    total.add(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                            itertools::EitherOrBoth::Right(right) => {
                                for old in right {
                                    let summary = Summary::from_old(old.0, old.1, old.2);
                                    total.add(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                        }
                    }
                }
                itertools::EitherOrBoth::Left(left) => {
                    for new in left.into_iter().flatten() {
                        let summary = Summary::from_new(new.0, new.1, new.2);
                        total.add(&summary.metrics_summary);
                        summaries.push(summary);
                    }
                }
                itertools::EitherOrBoth::Right(right) => {
                    for old in right.into_iter().flatten() {
                        let summary = Summary::from_old(old.0, old.1, old.2);
                        total.add(&summary.metrics_summary);
                        summaries.push(summary);
                    }
                }
            }
        }

        Self { summaries, total }
    }

    pub fn has_multiple(&self) -> bool {
        self.summaries.len() > 1
    }
}

impl From<Summaries> for ToolRun {
    fn from(value: Summaries) -> Self {
        let segments = value.summaries.into_iter().map(Into::into).collect();
        Self {
            total: ToolMetricSummary::CallgrindSummary(value.total),
            segments,
        }
    }
}

impl From<&Summaries> for ToolRun {
    fn from(value: &Summaries) -> Self {
        value.clone().into()
    }
}

impl Summary {
    pub fn new(
        details: EitherOrBoth<(PathBuf, CallgrindProperties)>,
        metrics_summary: MetricsSummary,
    ) -> Self {
        Self {
            details,
            metrics_summary,
        }
    }

    pub fn from_new(path: PathBuf, properties: CallgrindProperties, costs: Metrics) -> Self {
        Self {
            details: EitherOrBoth::Left((path, properties)),
            metrics_summary: MetricsSummary::new(EitherOrBoth::Left(costs)),
        }
    }

    pub fn from_old(path: PathBuf, properties: CallgrindProperties, costs: Metrics) -> Self {
        Self {
            details: EitherOrBoth::Right((path, properties)),
            metrics_summary: MetricsSummary::new(EitherOrBoth::Right(costs)),
        }
    }

    pub fn from_new_and_old(
        new: (PathBuf, CallgrindProperties, Metrics),
        old: (PathBuf, CallgrindProperties, Metrics),
    ) -> Self {
        Self {
            details: EitherOrBoth::Both((new.0, new.1), (old.0, old.1)),
            metrics_summary: MetricsSummary::new(EitherOrBoth::Both(new.2, old.2)),
        }
    }
}

impl From<Summary> for ToolRunSegment {
    fn from(value: Summary) -> Self {
        match value.details {
            EitherOrBoth::Left((new_path, new_props)) => ToolRunSegment {
                metrics_summary: ToolMetricSummary::CallgrindSummary(value.metrics_summary),
                details: EitherOrBoth::Left(new_props.into_info(&new_path)),
            },
            EitherOrBoth::Right((old_path, old_props)) => ToolRunSegment {
                metrics_summary: ToolMetricSummary::CallgrindSummary(value.metrics_summary),
                details: EitherOrBoth::Right(old_props.into_info(&old_path)),
            },
            EitherOrBoth::Both((new_path, new_props), (old_path, old_props)) => ToolRunSegment {
                metrics_summary: ToolMetricSummary::CallgrindSummary(value.metrics_summary),
                details: EitherOrBoth::Both(
                    new_props.into_info(&new_path),
                    old_props.into_info(&old_path),
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use EventKind::*;

    use super::*;

    fn cachesim_costs(costs: [u64; 9]) -> Metrics {
        Metrics::with_metric_kinds([
            (Ir, costs[0]),
            (Dr, costs[1]),
            (Dw, costs[2]),
            (I1mr, costs[3]),
            (D1mr, costs[4]),
            (D1mw, costs[5]),
            (ILmr, costs[6]),
            (DLmr, costs[7]),
            (DLmw, costs[8]),
        ])
    }

    #[rstest]
    fn test_regression_check_when_old_is_none() {
        let regression = RegressionConfig::default();
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
    fn test_regression_check_when_old_is_some(
        #[case] limits: Vec<(EventKind, f64)>,
        #[case] new: [u64; 9],
        #[case] old: [u64; 9],
        #[case] expected: Vec<(EventKind, u64, u64, f64, f64)>,
    ) {
        let regression = RegressionConfig {
            limits,
            ..Default::default()
        };

        let new = cachesim_costs(new);
        let old = cachesim_costs(old);
        let summary = MetricsSummary::new(EitherOrBoth::Both(new, old));
        let expected = expected
            .iter()
            .map(|(e, n, o, d, l)| CallgrindRegression {
                event_kind: *e,
                new: *n,
                old: *o,
                diff_pct: *d,
                limit: *l,
            })
            .collect::<Vec<CallgrindRegression>>();

        assert_eq!(regression.check(&summary), expected);
    }
}
