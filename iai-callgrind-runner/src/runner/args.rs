// spell-checker: ignore totalbytes totalblocks writeback writebackbehaviour
//! The command-line arguments of cargo bench as in ARGS of `cargo bench -- ARGS`

use std::fmt::Display;
use std::hash::Hash;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;

use clap::builder::BoolishValueParser;
use clap::{ArgAction, Parser};
use indexmap::{indexset, IndexMap, IndexSet};
use strum::IntoEnumIterator;

use super::cachegrind::regression::CachegrindRegressionConfig;
use super::callgrind::regression::CallgrindRegressionConfig;
use super::dhat::regression::DhatRegressionConfig;
use super::format::OutputFormatKind;
use super::metrics::{Metric, TypeChecker};
use super::summary::{BaselineName, SummaryFormat};
use super::tool::regression::ToolRegressionConfig;
use crate::api::{
    CachegrindMetric, CachegrindMetrics, CallgrindMetrics, DhatMetric, DhatMetrics, ErrorMetric,
    EventKind, RawArgs, ValgrindTool,
};

// Utility for complex types intended to be used during the parsing of the command-line arguments
type Limits<T> = (IndexMap<T, f64>, IndexMap<T, Metric>);
type ParsedMetrics<T> = Result<Vec<(T, Option<Metric>)>, String>;

/// A filter for benchmarks
///
/// # Developer Notes
///
/// This enum is used instead of a plain `String` for possible future usages to filter by benchmark
/// ids, group name, file name etc.
#[derive(Debug, Clone)]
pub enum BenchmarkFilter {
    /// The name of the benchmark
    Name(String),
}

/// The `NoCapture` options for the command-line argument --nocapture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoCapture {
    /// Don't capture any output
    True,
    /// Capture all output
    False,
    /// Don't capture `stderr`
    Stderr,
    /// Don't capture `stdout`
    Stdout,
}

// TODO: Sort with display order and then remove `clippy::arbitrary_source_item_ordering` from
// allow
/// The command line arguments the user provided after `--` when running cargo bench
///
/// These arguments are not the command line arguments passed to `iai-callgrind-runner`. We collect
/// the command line arguments in the `iai-callgrind::main!` macro without the binary as first
/// argument, that's why `no_binary_name` is set to `true`.
#[allow(
    clippy::partial_pub_fields,
    clippy::struct_excessive_bools,
    clippy::arbitrary_source_item_ordering
)]
#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "High-precision and consistent benchmarking framework/harness for Rust

Boolish command line arguments take also one of `y`, `yes`, `t`, `true`, `on`, `1`
instead of `true` and one of `n`, `no`, `f`, `false`, `off`, and `0` instead of
`false`",
    after_help = "  Exit codes:
      0: Success
      1: All other errors
      2: Parsing command-line arguments failed
      3: One or more regressions occurred
    ",
    long_about = None,
    no_binary_name = true,
    override_usage= "cargo bench ... [BENCHNAME] -- [OPTIONS]",
    max_term_width = 100
)]
pub struct CommandLineArgs {
    /// The following arguments are accepted by the rust libtest harness and ignored by us
    ///
    /// Further details in <https://doc.rust-lang.org/rustc/tests/index.html#cli-arguments> or by
    /// running `cargo test -- --help`
    #[arg(long = "include-ignored", hide = true, action = ArgAction::SetTrue, required = false)]
    _include_ignored: bool,

    #[arg(long = "ignored", hide = true, action = ArgAction::SetTrue, required = false)]
    _ignored: bool,

    #[arg(long = "force-run-in-process", hide = true, action = ArgAction::SetTrue, required = false)]
    _force_run_in_process: bool,

    #[arg(long = "exclude-should-panic", hide = true, action = ArgAction::SetTrue, required = false)]
    _exclude_should_panic: bool,

    #[arg(long = "test", hide = true, action = ArgAction::SetTrue, required = false)]
    _test: bool,

    /// `--bench` also shows up as last argument set by `cargo bench` even if not explicitly given
    #[arg(long = "bench", hide = true, action = ArgAction::SetTrue, required = false)]
    _bench: bool,

    #[arg(long = "logfile", hide = true, required = false, num_args = 0..)]
    _logfile: Vec<String>,

    #[arg(long = "test-threads", hide = true, required = false, num_args = 0..)]
    _test_threads: Vec<String>,

    #[arg(long = "skip", hide = true, required = false, num_args = 0..)]
    _skip: Vec<String>,

    #[arg(long = "quiet", short = 'q', hide = true, action = ArgAction::SetTrue, required = false)]
    _quiet: bool,

    #[arg(long = "exact", hide = true, action = ArgAction::SetTrue, required = false)]
    _exact: bool,

    #[arg(long = "color", hide = true, required = false, num_args = 0..)]
    _color: Vec<String>,

    #[arg(long = "format", hide = true, required = false, num_args = 0..)]
    _format: Vec<String>,

    #[arg(long = "show-output", hide = true, action = ArgAction::SetTrue, required = false)]
    _show_output: bool,

    #[arg(short = 'Z', hide = true, required = false, num_args = 0..)]
    _unstable_options: Vec<String>,

    #[arg(long = "report-time", hide = true, action = ArgAction::SetTrue, required = false)]
    _report_time: bool,

    #[arg(long = "ensure-time", hide = true, action = ArgAction::SetTrue, required = false)]
    _ensure_time: bool,

    #[arg(long = "shuffle", hide = true, action = ArgAction::SetTrue, required = false)]
    _shuffle: bool,

    // This is the last of the ignored libtest args
    #[arg(long = "shuffle-seed", hide = true, required = false, num_args = 0..)]
    _shuffle_seed: Vec<String>,

    /// If specified, only run benches containing this string in their names
    ///
    /// Note that a benchmark name might differ from the benchmark file name.
    #[arg(name = "BENCHNAME", num_args = 0..=1, env = "IAI_CALLGRIND_FILTER")]
    pub filter: Option<BenchmarkFilter>,

    /// Print a list of all benchmarks. With this argument no benchmarks are executed.
    ///
    /// The output format is intended to be the same as the output format of the libtest harness.
    /// However, future changes of the output format by cargo might not be incorporated into
    /// iai-callgrind. As a consequence, it is not considered safe to rely on the output in
    /// scripts.
    #[arg(
        long = "list",
        default_missing_value = "true",
        default_value = "false",
        num_args = 0..=1,
        require_equals = true,
        value_parser = BoolishValueParser::new(),
        action = ArgAction::Set,
        env = "IAI_CALLGRIND_LIST"
    )]
    pub list: bool,

    /// The default tool used to run the benchmarks
    ///
    /// The standard tool to run the benchmarks is callgrind but can be overridden with this
    /// option. Any valgrind tool can be used:
    ///   * callgrind
    ///   * cachegrind
    ///   * dhat
    ///   * memcheck
    ///   * helgrind
    ///   * drd
    ///   * massif
    ///   * exp-bbv
    ///
    /// This argument matches the tool case-insensitive. Note that using cachegrind with this
    /// option to benchmark library functions needs adjustments to the benchmarking functions
    /// with client-requests to measure the counts correctly. If you want to switch permanently
    /// to cachegrind, it is usually better to activate the `cachegrind` feature of
    /// iai-callgrind in your Cargo.toml. However, setting a tool with this option overrides
    /// cachegrind set with the iai-callgrind feature. See the guide for all details.
    #[arg(
        long = "default-tool",
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_DEFAULT_TOOL"
    )]
    pub default_tool: Option<ValgrindTool>,

    /// A comma separated list of tools to run additionally to callgrind or another default tool
    ///
    /// The tools specified here take precedence over the tools in the benchmarks. The valgrind
    /// tools which are allowed here are the same as the ones listed in the documentation of
    /// --default-tool.
    ///
    /// Examples
    ///   * --tools dhat
    ///   * --tools memcheck,drd
    #[arg(
        long = "tools",
        num_args = 1..,
        value_delimiter = ',',
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_TOOLS"
    )]
    pub tools: Vec<ValgrindTool>,

    /// The command-line arguments to pass through to all tools
    ///
    /// The core valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options> which are recognized
    /// by all tools. More specific arguments for example set with --callgrind-args override the
    /// arguments with the same name specified with this option.
    ///
    /// Examples:
    ///   * --valgrind-args=--time-stamp=yes
    ///   * --valgrind-args='--error-exitcode=202 --num-callers=50'
    #[arg(
        long = "valgrind-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_VALGRIND_ARGS"
    )]
    pub valgrind_args: Option<RawArgs>,

    /// The command-line arguments to pass through to Callgrind
    ///
    /// <https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options> and the core valgrind
    /// command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>. Note that not all
    /// command-line arguments are supported especially the ones which change output paths.
    /// Unsupported arguments will be ignored printing a warning.
    ///
    /// Examples:
    ///   * --callgrind-args=--dump-instr=yes
    ///   * --callgrind-args='--dump-instr=yes --collect-systime=yes'
    #[arg(
        long = "callgrind-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_CALLGRIND_ARGS"
    )]
    pub callgrind_args: Option<RawArgs>,

    /// The command-line arguments to pass through to Cachegrind
    ///
    /// <https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --cachegrind-args=--intr-at-start=no
    ///   * --cachegrind-args='--branch-sim=yes --instr-at-start=no'
    #[arg(
        long = "cachegrind-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_CACHEGRIND_ARGS"
    )]
    pub cachegrind_args: Option<RawArgs>,

    /// The command-line arguments to pass through to DHAT
    ///
    /// <https://valgrind.org/docs/manual/dh-manual.html#dh-manual.options>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --dhat-args=--mode=ad-hoc
    #[arg(
        long = "dhat-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_DHAT_ARGS"
    )]
    pub dhat_args: Option<RawArgs>,

    /// The command-line arguments to pass through to Memcheck
    ///
    /// <https://valgrind.org/docs/manual/mc-manual.html#mc-manual.options>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --memcheck-args=--leak-check=full
    ///   * --memcheck-args='--leak-check=yes --show-leak-kinds=all'
    #[arg(
        long = "memcheck-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_MEMCHECK_ARGS"
    )]
    pub memcheck_args: Option<RawArgs>,

    /// The command-line arguments to pass through to Helgrind
    ///
    /// <https://valgrind.org/docs/manual/hg-manual.html#hg-manual.options>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --helgrind-args=--free-is-write=yes
    ///   * --helgrind-args='--conflict-cache-size=100000 --free-is-write=yes'
    #[arg(
        long = "helgrind-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_HELGRIND_ARGS"
    )]
    pub helgrind_args: Option<RawArgs>,

    /// The command-line arguments to pass through to DRD
    ///
    /// <https://valgrind.org/docs/manual/drd-manual.html#drd-manual.options>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --drd-args=--exclusive-threshold=100
    ///   * --drd-args='--exclusive-threshold=100 --free-is-write=yes'
    #[arg(
        long = "drd-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_DRD_ARGS"
    )]
    pub drd_args: Option<RawArgs>,

    /// The command-line arguments to pass through to Massif
    ///
    /// <https://valgrind.org/docs/manual/ms-manual.html#ms-manual.options>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --massif-args=--heap=no
    ///   * --massif-args='--heap=no --threshold=2.0'
    #[arg(
        long = "massif-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_MASSIF_ARGS"
    )]
    pub massif_args: Option<RawArgs>,

    /// The command-line arguments to pass through to the experimental BBV
    ///
    /// <https://valgrind.org/docs/manual/bbv-manual.html#bbv-manual.usage>. See also the description
    /// for --callgrind-args for more details and restrictions.
    ///
    /// Examples:
    ///   * --bbv-args=--interval-size=10000
    ///   * --bbv-args='--interval-size=10000 --instr-count-only=yes'
    #[arg(
        long = "bbv-args",
        value_parser = parse_args,
        num_args = 1,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_BBV_ARGS"
    )]
    pub bbv_args: Option<RawArgs>,

    #[rustfmt::skip]
    #[allow(clippy::doc_markdown)]
    /// Set performance regression limits for specific `EventKinds`
    ///
    /// This is a `,` separate list of EventKind=limit or CallgrindMetrics=limit (key=value) pairs
    /// with the limit being a soft limit if the number suffixed with a `%` or a hard limit if it
    /// is a bare number. It is possible to specify hard and soft limits in one go with the `|`
    /// operator (e.g. `ir=10%|10000`). Groups (CallgrindMetrics) are prefixed with `@`. List of
    /// allowed groups and events with their abbreviations:
    ///
    /// group ::= "@" ( "default"
    ///               | "all"
    ///               | ("cachemisses" | "misses" | "ms")
    ///               | ("cachemissrates" | "missrates" | "mr")
    ///               | ("cachehits" | "hits" | "hs")
    ///               | ("cachehitrates" | "hitrates" | "hr")
    ///               | ("cachesim" | "cs")
    ///               | ("cacheuse" | "cu")
    ///               | ("systemcalls" | "syscalls" | "sc")
    ///               | ("branchsim" | "bs")
    ///               | ("writebackbehaviour" | "writeback" | "wb")
    ///               )
    /// event ::= EventKind
    ///
    /// See the guide (https://iai-callgrind.github.io/iai-callgrind/latest/html/regressions.html)
    /// for more details, the docs of `CallgrindMetrics`
    /// (<https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CallgrindMetrics.html>) and
    /// `EventKind` <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html> for a
    /// list of metrics and groups with their members.
    ///
    /// A performance regression check for an `EventKind` fails if the limit is exceeded. If
    /// limits are defined and one or more regressions have occurred during the benchmark run,
    /// the whole benchmark is considered to have failed and the program exits with error and
    /// exit code `3`.
    ///
    /// Examples:
    /// * --callgrind-limits='ir=5.0%'
    /// * --callgrind-limits='ir=10000,EstimatedCycles=10%'
    /// * --callgrind-limits='@all=10%,ir=5%|10000'
    #[arg(
        long = "callgrind-limits",
        num_args = 1,
        verbatim_doc_comment,
        value_parser = parse_callgrind_limits,
        env = "IAI_CALLGRIND_CALLGRIND_LIMITS",
    )]
    pub callgrind_limits: Option<ToolRegressionConfig>,

    #[rustfmt::skip]
    #[allow(clippy::doc_markdown)]
    /// Set performance regression limits for specific cachegrind metrics
    ///
    /// This is a `,` separate list of CachegrindMetric=limit or CachegrindMetrics=limit
    /// (key=value) pairs. See the description of --callgrind-limits for the details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetrics.html>
    /// respectively <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetric.html>
    /// for valid metrics and group members.
    ///
    /// See the the guide
    /// (https://iai-callgrind.github.io/iai-callgrind/latest/html/regressions.html) for all
    /// details or replace the format spec in `--callgrind-limits` or with the following:
    ///
    /// group ::= "@" ( "default"
    ///               | "all"
    ///               | ("cachemisses" | "misses" | "ms")
    ///               | ("cachemissrates" | "missrates" | "mr")
    ///               | ("cachehits" | "hits" | "hs")
    ///               | ("cachehitrates" | "hitrates" | "hr")
    ///               | ("cachesim" | "cs")
    ///               | ("branchsim" | "bs")
    ///               )
    /// event ::= CachegrindMetric
    ///
    /// Examples:
    /// * --cachegrind-limits='ir=0.0%'
    /// * --cachegrind-limits='ir=10000,EstimatedCycles=10%'
    /// * --cachegrind-limits='@all=10%,ir=10000,EstimatedCycles=10%'
    #[arg(
        long = "cachegrind-limits",
        num_args = 1,
        verbatim_doc_comment,
        value_parser = parse_cachegrind_limits,
        env = "IAI_CALLGRIND_CACHEGRIND_LIMITS",
    )]
    pub cachegrind_limits: Option<ToolRegressionConfig>,

    #[allow(clippy::doc_markdown)]
    /// Set performance regression limits for specific dhat metrics
    ///
    /// This is a `,` separate list of DhatMetrics=limit or DhatMetric=limit (key=value) pairs. See
    /// the description of --callgrind-limits for the details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetrics.html> respectively
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetric.html> for valid
    /// metrics and group members.
    ///
    /// See the the guide
    /// (https://iai-callgrind.github.io/iai-callgrind/latest/html/regressions.html) for all
    /// details or replace the format spec in `--callgrind-limits` or with the following:
    ///
    /// group ::= "@" ( "default" | "all" )
    /// event ::=   ( "totalunits" | "tun" )
    ///           | ( "totalevents" | "tev" )
    ///           | ( "totalbytes" | "tb" )
    ///           | ( "totalblocks" | "tbk" )
    ///           | ( "attgmaxbytes" | "gb" )
    ///           | ( "attgmaxblocks" | "gbk" )
    ///           | ( "attendbytes" | "eb" )
    ///           | ( "attendblocks" | "ebk" )
    ///           | ( "readsbytes" | "rb" )
    ///           | ( "writesbytes" | "wb" )
    ///           | ( "totallifetimes" | "tl" )
    ///           | ( "maximumbytes" | "mb" )
    ///           | ( "maximumblocks" | "mbk" )
    ///
    /// `events` with a long name have their allowed abbreviations placed in the same parentheses.
    ///
    /// Examples:
    /// * --dhat-limits='totalbytes=0.0%'
    /// * --dhat-limits='totalbytes=10000,totalblocks=5%'
    /// * --dhat-limits='@all=10%,totalbytes=5000,totalblocks=5%'
    #[arg(
        long = "dhat-limits",
        num_args = 1,
        verbatim_doc_comment,
        value_parser = parse_dhat_limits,
        env = "IAI_CALLGRIND_DHAT_LIMITS",
    )]
    pub dhat_limits: Option<ToolRegressionConfig>,

    /// If true, the first failed performance regression check fails the whole benchmark run
    ///
    /// Note that if --regression-fail-fast is set to true, no summary is printed.
    #[arg(
        long = "regression-fail-fast",
        default_missing_value = "true",
        num_args = 0..=1,
        require_equals = true,
        value_parser = BoolishValueParser::new(),
        env = "IAI_CALLGRIND_REGRESSION_FAIL_FAST",
    )]
    pub regression_fail_fast: Option<bool>,

    /// Define the cachegrind metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of cachegrind metric groups and event kinds which are allowed
    /// to appear in the terminal output of cachegrind.
    ///
    /// See `--callgrind-limits` for more details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetrics.html>
    /// respectively
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetric.html> for valid
    /// metrics and group members.
    ///
    /// The `group` names, their abbreviations if present and `event` kinds are exactly the same as
    /// described in the `--cachegrind-limits` option.
    ///
    /// Examples:
    /// * --cachegrind-metrics='ir' to show only `Instructions`
    /// * --cachegrind-metrics='@all' to show all possible cachegrind metrics
    /// * --cachegrind-metrics='@default,@mr' to show cache miss rates in addition to the defaults
    #[arg(
        long = "cachegrind-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_cachegrind_metrics,
        env = "IAI_CALLGRIND_CACHEGRIND_METRICS",
    )]
    pub cachegrind_metrics: Option<IndexSet<CachegrindMetric>>,

    /// Define the callgrind metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of callgrind metric groups and event kinds which are allowed
    /// to appear in the terminal output of callgrind. Group names need to be prefixed with '@'.
    /// The order matters and the callgrind metrics are shown in their insertion order of this
    /// option. More precisely, in case of duplicate metrics, the first inserted one wins.
    ///
    /// The `group` names, their abbreviations if present and `event` kinds are exactly the same as
    /// described in the `--callgrind-limits` option.
    ///
    /// For a list of valid metrics, groups and their members see the docs of `CallgrindMetrics`
    /// (<https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CallgrindMetrics.html>) and
    /// `EventKind` <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html>.
    ///
    /// Note that setting the metrics here does not imply that these metrics are actually
    /// collected. This option just sets the order and appearance of metrics in case they are
    /// collected. To activate the collection of specific metrics you need to use
    /// `--callgrind-args`.
    ///
    /// Examples:
    /// * --callgrind-metrics='ir' to show only `Instructions`
    /// * --callgrind-metrics='@all' to show all possible callgrind metrics
    /// * --callgrind-metrics='@default,@mr' to show cache miss rates in addition to the defaults
    #[arg(
        long = "callgrind-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_callgrind_metrics,
        env = "IAI_CALLGRIND_CALLGRIND_METRICS",
    )]
    pub callgrind_metrics: Option<IndexSet<EventKind>>,

    /// Define the dhat metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of dhat metric groups and event kinds which are allowed to
    /// appear in the terminal output of dhat.
    ///
    /// See `--callgrind-metrics` for more details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetrics.html> respectively
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetric.html> for valid metrics
    /// and group members.
    ///
    /// The `group` names, their abbreviations if present and `event` kinds are exactly the same as
    /// described in the `--dhat-limits` option.
    ///
    /// Examples:
    /// * --dhat-metrics='totalbytes' to show only `Total Bytes`
    /// * --dhat-metrics='@all' to show all possible dhat metrics
    /// * --dhat-metrics='@default,mb' to show maximum bytes in addition to the defaults
    #[arg(
        long = "dhat-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_dhat_metrics,
        env = "IAI_CALLGRIND_DHAT_METRICS",
    )]
    pub dhat_metrics: Option<IndexSet<DhatMetric>>,

    /// Define the memcheck error metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of error metrics which are allowed to appear in the terminal
    /// output of memcheck.
    ///
    /// Since this is a very small set of metrics, there is only one `group`: `@all`
    ///
    /// group ::= "@all"
    /// event ::=   ( "errors" | "err" )
    ///           | ( "contexts" | "ctx" )
    ///           | ( "suppressederrors" | "serr")
    ///           | ( "suppressedcontexts" | "sctx" )
    ///
    /// See `--callgrind-metrics` for more details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid
    /// metrics.
    ///
    /// Examples:
    /// * --memcheck-metrics='errors' to show only `Errors`
    /// * --memcheck-metrics='@all' to show all possible error metrics (the default)
    /// * --memcheck-metrics='err,ctx' to show only errors and contexts
    #[arg(
        long = "memcheck-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_memcheck_metrics,
        env = "IAI_CALLGRIND_MEMCHECK_METRICS",
    )]
    pub memcheck_metrics: Option<IndexSet<ErrorMetric>>,

    /// Define the drd error metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of error metrics which are allowed to appear in the terminal
    /// output of drd. The `group` and `event` are the same as for `--memcheck-metrics`.
    ///
    /// See `--callgrind-metrics` for more details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid
    /// error metrics.
    ///
    /// Since this is a very small set of metrics, there is only one `group`: `@all`
    ///
    /// Examples:
    /// * --drd-metrics='errors' to show only `Errors`
    /// * --drd-metrics='@all' to show all possible error metrics (the default)
    /// * --drd-metrics='err,ctx' to show only errors and contexts
    #[arg(
        long = "drd-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_drd_metrics,
        env = "IAI_CALLGRIND_DRD_METRICS",
    )]
    pub drd_metrics: Option<IndexSet<ErrorMetric>>,

    /// Define the helgrind error metrics and the order in which they are displayed
    ///
    /// This is a `,`-separated list of error metrics which are allowed to appear in the terminal
    /// output of helgrind. The `group` and `event` are the same as for `--memcheck-metrics`.
    ///
    /// See `--callgrind-metrics` for more details and
    /// <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid
    /// error metrics.
    ///
    /// Examples:
    /// * --helgrind-metrics='errors' to show only `Errors`
    /// * --helgrind-metrics='@all' to show all possible error metrics (the default)
    /// * --helgrind-metrics='err,ctx' to show only errors and contexts
    #[arg(
        long = "helgrind-metrics",
        num_args = 1..,
        required = false,
        verbatim_doc_comment,
        value_parser = parse_helgrind_metrics,
        env = "IAI_CALLGRIND_HELGRIND_METRICS",
    )]
    pub helgrind_metrics: Option<IndexSet<ErrorMetric>>,

    /// Compare against this baseline if present and then overwrite it
    #[arg(
        long = "save-baseline",
        default_missing_value = "default",
        num_args = 0..=1,
        require_equals = true,
        conflicts_with_all = &["baseline", "LOAD_BASELINE"],
        env = "IAI_CALLGRIND_SAVE_BASELINE",
    )]
    pub save_baseline: Option<BaselineName>,

    /// Compare against this baseline if present but do not overwrite it
    #[arg(
        long = "baseline",
        default_missing_value = "default",
        num_args = 0..=1,
        require_equals = true,
        env = "IAI_CALLGRIND_BASELINE"
    )]
    pub baseline: Option<BaselineName>,

    /// Load this baseline as the new data set instead of creating a new one
    #[clap(
        id = "LOAD_BASELINE",
        long = "load-baseline",
        requires = "baseline",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "default",
        env = "IAI_CALLGRIND_LOAD_BASELINE"
    )]
    pub load_baseline: Option<BaselineName>,

    /// Allow ASLR (Address Space Layout Randomization)
    ///
    /// If possible, ASLR is disabled on platforms that support it (linux, freebsd) because ASLR
    /// could noise up the callgrind cache simulation results a bit. Setting this option to true
    /// runs all benchmarks with ASLR enabled.
    ///
    /// See also <https://docs.kernel.org/admin-guide/sysctl/kernel.html?highlight=randomize_va_space#randomize-va-space>
    #[arg(
        long = "allow-aslr",
        default_missing_value = "true",
        num_args = 0..=1,
        require_equals = true,
        value_parser = BoolishValueParser::new(),
        env = "IAI_CALLGRIND_ALLOW_ASLR",
    )]
    pub allow_aslr: Option<bool>,

    /// Save a machine-readable summary of each benchmark run in json format next to the usual
    /// benchmark output
    #[arg(
        long = "save-summary",
        value_enum,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "json",
        env = "IAI_CALLGRIND_SAVE_SUMMARY"
    )]
    pub save_summary: Option<SummaryFormat>,

    /// The terminal output format in default human-readable format or in machine-readable json
    /// format
    ///
    /// # The JSON Output Format
    ///
    /// The json terminal output schema is the same as the schema with the `--save-summary`
    /// argument when saving to a `summary.json` file. All other output than the json output goes
    /// to stderr and only the summary output goes to stdout. When not printing pretty json, each
    /// line is a dictionary summarizing a single benchmark. You can combine all lines
    /// (benchmarks) into an array for example with `jq`
    ///
    /// `cargo bench -- --output-format=json | jq -s`
    ///
    /// which transforms `{...}\n{...}` into `[{...},{...}]`
    #[arg(
        long = "output-format",
        value_enum,
        required = false,
        default_value = "default",
        num_args = 1,
        env = "IAI_CALLGRIND_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormatKind,

    #[rustfmt::skip]
    /// Show changes only when they are above the `tolerance` level
    ///
    /// If no value is specified, the default value of `0.000_009_999_999_999_999_999` is based on
    /// the number of decimal places of the percentages displayed in the terminal output in case of
    /// differences.
    ///
    /// Negative tolerance values are converted to their absolute value.
    ///
    /// Examples:
    /// * --tolerance (applies the default value)
    /// * --tolerance=0.1 (set the tolerance level to `0.1`)
    #[arg(
        long = "tolerance",
        default_missing_value = "0.000009999999999999999",
        num_args = 0..=1,
        require_equals = true,
        verbatim_doc_comment,
        env = "IAI_CALLGRIND_TOLERANCE"
    )]
    pub tolerance: Option<f64>,

    /// Separate iai-callgrind benchmark output files by target
    ///
    /// The default output path for files created by iai-callgrind and valgrind during the
    /// benchmark is
    ///
    /// `target/iai/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`.
    ///
    /// This can be problematic if you're running the benchmarks not only for a
    /// single target because you end up comparing the benchmark runs with the wrong targets.
    /// Setting this option changes the default output path to
    ///
    /// `target/iai/$TARGET/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`
    ///
    /// Although not as comfortable and strict, you could achieve a separation by target also with
    /// baselines and a combination of `--save-baseline=$TARGET` and `--baseline=$TARGET` if you
    /// prefer having all files of a single $BENCH in the same directory.
    #[arg(
        long = "separate-targets",
        default_missing_value = "true",
        default_value = "false",
        num_args = 0..=1,
        require_equals = true,
        value_parser = BoolishValueParser::new(),
        action = ArgAction::Set,
        env = "IAI_CALLGRIND_SEPARATE_TARGETS",
    )]
    pub separate_targets: bool,

    /// Specify the home directory of iai-callgrind benchmark output files
    ///
    /// All output files are per default stored under the `$PROJECT_ROOT/target/iai` directory.
    /// This option lets you customize this home directory, and it will be created if it
    /// doesn't exist.
    #[arg(long = "home", num_args = 1, env = "IAI_CALLGRIND_HOME")]
    pub home: Option<PathBuf>,

    /// Don't capture terminal output of benchmarks
    ///
    /// Possible values are one of [true, false, stdout, stderr].
    ///
    /// This option is currently restricted to the `callgrind` run of benchmarks. The output of
    /// additional tool runs like DHAT, Memcheck, ... is still captured, to prevent showing the
    /// same output of benchmarks multiple times. Use `IAI_CALLGRIND_LOG=info` to also show
    /// captured and logged output.
    ///
    /// If no value is given, the default missing value is `true` and doesn't capture stdout and
    /// stderr. Besides `true` or `false` you can specify the special values `stdout` or `stderr`.
    /// If `--nocapture=stdout` is given, the output to `stdout` won't be captured and the output
    /// to `stderr` will be discarded. Likewise, if `--nocapture=stderr` is specified, the
    /// output to `stderr` won't be captured and the output to `stdout` will be discarded.
    #[arg(
        long = "nocapture",
        required = false,
        default_missing_value = "true",
        default_value = "false",
        num_args = 0..=1,
        require_equals = true,
        value_parser = parse_nocapture,
        env = "IAI_CALLGRIND_NOCAPTURE"
    )]
    pub nocapture: NoCapture,

    /// Suppress the summary showing regressions and execution time at the end of a benchmark run
    ///
    /// Note, that a summary is only printed if the `--output-format` is not JSON.
    ///
    /// The summary described by `--nosummary` is different from `--save-summary` and they do not
    /// affect each other.
    #[arg(
        long = "nosummary",
        default_missing_value = "true",
        default_value = "false",
        num_args = 0..=1,
        require_equals = true,
        value_parser = BoolishValueParser::new(),
        action = ArgAction::Set,
        env = "IAI_CALLGRIND_NOSUMMARY"
    )]
    pub nosummary: bool,
}

impl BenchmarkFilter {
    /// Return true if the haystack contains the filter
    pub fn apply(&self, haystack: &str) -> bool {
        let Self::Name(name) = self;
        haystack.contains(name)
    }
}

impl FromStr for BenchmarkFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Name(s.to_owned()))
    }
}

impl NoCapture {
    /// Apply the `NoCapture` option to the [`Command`]
    pub fn apply(self, command: &mut Command) {
        match self {
            Self::True | Self::False => {}
            Self::Stderr => {
                command.stdout(Stdio::null()).stderr(Stdio::inherit());
            }
            Self::Stdout => {
                command.stdout(Stdio::inherit()).stderr(Stdio::null());
            }
        }
    }
}

// Convert the `metric` if it is present
//
// Used for example for hard limits to convert u64 values to f64 values if required.
fn convert_metric<T: Display + TypeChecker + Copy>(
    metric_kind: T,
    metric: Option<Metric>,
) -> Result<(T, Option<Metric>), String> {
    if let Some(metric) = metric {
        metric
            .try_convert(metric_kind)
            .ok_or_else(|| {
                format!(
                    "Invalid hard limit for '{metric_kind}': Expected an integer (e.g. '10'). If \
                     you wanted this value to be a soft limit use the '%' suffix (e.g. '4.0%' or \
                     '4%')"
                )
            })
            .map(|(t, m)| (t, Some(m)))
    } else {
        Ok((metric_kind, None))
    }
}

/// This function parses a space separated list of raw argument strings into [`crate::api::RawArgs`]
fn parse_args(value: &str) -> Result<RawArgs, String> {
    shlex::split(value)
        .ok_or_else(|| "Failed to split args".to_owned())
        .map(RawArgs::new)
}

/// Same as `parse_callgrind_limits` but for cachegrind
fn parse_cachegrind_limits(value: &str) -> Result<ToolRegressionConfig, String> {
    let (soft_limits, hard_limits) = parse_limits(value, |key, metric| {
        let metrics = key
            .parse::<CachegrindMetrics>()
            .map_err(|error| error.to_string())?;
        IndexSet::from(metrics)
            .into_iter()
            .map(|metric_kind| convert_metric(metric_kind, metric))
            .collect::<ParsedMetrics<CachegrindMetric>>()
    })?;

    let config = ToolRegressionConfig::Cachegrind(CachegrindRegressionConfig {
        soft_limits: soft_limits.into_iter().collect(),
        hard_limits: hard_limits.into_iter().collect(),
        ..Default::default()
    });

    Ok(config)
}

/// Parse the cachegrind metrics
fn parse_cachegrind_metrics(value: &str) -> Result<IndexSet<CachegrindMetric>, String> {
    parse_tool_metrics(value, |item| {
        item.parse::<CachegrindMetrics>()
            .map(IndexSet::from)
            .map_err(|error| error.to_string())
    })
}

/// Parse the callgrind limits from the command-line
///
/// This method (and the other `parse_dhat_limits`, ...) parses soft and hard limits in one go. The
/// format is described in the --help message above in [`CommandLineArgs`].
///
/// In order to avoid back and forth conversions between `api::ToolRegressionConfig` and
/// `tool::ToolRegressionConfig` we parse the `tool::ToolRegressionConfig` directly.
fn parse_callgrind_limits(value: &str) -> Result<ToolRegressionConfig, String> {
    let (soft_limits, hard_limits) = parse_limits(value, |key, metric| {
        let metrics = key
            .parse::<CallgrindMetrics>()
            .map_err(|error| error.to_string())?;
        IndexSet::from(metrics)
            .into_iter()
            .map(|event_kind| convert_metric(event_kind, metric))
            .collect::<ParsedMetrics<EventKind>>()
    })?;

    let config = ToolRegressionConfig::Callgrind(CallgrindRegressionConfig {
        soft_limits: soft_limits.into_iter().collect(),
        hard_limits: hard_limits.into_iter().collect(),
        ..Default::default()
    });

    Ok(config)
}

/// Parse the callgrind metrics
fn parse_callgrind_metrics(value: &str) -> Result<IndexSet<EventKind>, String> {
    parse_tool_metrics(value, |item| {
        item.parse::<CallgrindMetrics>()
            .map(IndexSet::from)
            .map_err(|error| error.to_string())
    })
}

/// Same as `parse_callgrind_limits` but for dhat
fn parse_dhat_limits(value: &str) -> Result<ToolRegressionConfig, String> {
    let (soft_limits, hard_limits) = parse_limits(value, |key, metric| {
        let metrics = key
            .parse::<DhatMetrics>()
            .map_err(|error| error.to_string())?;
        IndexSet::from(metrics)
            .into_iter()
            .map(|metric_kind| convert_metric(metric_kind, metric))
            .collect::<ParsedMetrics<DhatMetric>>()
    })?;

    let config = ToolRegressionConfig::Dhat(DhatRegressionConfig {
        soft_limits: soft_limits.into_iter().collect(),
        hard_limits: hard_limits.into_iter().collect(),
        ..Default::default()
    });

    Ok(config)
}

/// Parse the DHAT metrics
fn parse_dhat_metrics(value: &str) -> Result<IndexSet<DhatMetric>, String> {
    parse_tool_metrics(value, |item| {
        item.parse::<DhatMetrics>()
            .map(IndexSet::from)
            .map_err(|error| error.to_string())
    })
}

/// Parse the DRD metrics as error metrics
fn parse_drd_metrics(value: &str) -> Result<IndexSet<ErrorMetric>, String> {
    parse_tool_metrics(value, parse_error_metrics)
}

fn parse_error_metrics(item: &str) -> Result<IndexSet<ErrorMetric>, String> {
    if let Some(prefix) = item.strip_prefix('@') {
        if prefix == "all" {
            Ok(ErrorMetric::iter().fold(IndexSet::new(), |mut acc, elem| {
                acc.insert(elem);
                acc
            }))
        } else {
            Err(format!("Invalid error metric group: '{item}"))
        }
    } else {
        let metric = item
            .parse::<ErrorMetric>()
            .map_err(|error| error.to_string())?;
        Ok(indexset! { metric })
    }
}

/// Parse the helgrind metrics as error metrics
fn parse_helgrind_metrics(value: &str) -> Result<IndexSet<ErrorMetric>, String> {
    parse_tool_metrics(value, parse_error_metrics)
}

fn parse_limits<T: Eq + Hash>(
    value: &str,
    parse_metrics: fn(&str, Option<Metric>) -> ParsedMetrics<T>,
) -> Result<Limits<T>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("No limits found: At least one limit must be present".to_owned());
    }

    let mut soft_limits = IndexMap::new();
    let mut hard_limits = IndexMap::new();

    for item in value.split(',') {
        let item = item.trim();

        if let Some((key, value)) = item.split_once('=') {
            let (key, value) = (key.trim(), value.trim());
            for split in value.split('|') {
                let split = split.trim();

                if let Some(prefix) = split.strip_suffix('%') {
                    let pct = prefix.parse::<f64>().map_err(|error| -> String {
                        format!("Invalid soft limit for '{key}': {error}")
                    })?;
                    let metric_kinds = parse_metrics(key, None)?;
                    for (metric_kind, _) in metric_kinds {
                        soft_limits.insert(metric_kind, pct);
                    }
                } else {
                    let metric = split.parse::<Metric>().map_err(|error| -> String {
                        format!("Invalid hard limit for '{key}': {error}")
                    })?;
                    let metric_kinds = parse_metrics(key, Some(metric))?;
                    for (metric_kind, new_metric) in metric_kinds {
                        if let Some(new_metric) = new_metric {
                            hard_limits.insert(metric_kind, new_metric);
                        } else {
                            hard_limits.insert(metric_kind, metric);
                        }
                    }
                }
            }
        } else {
            return Err(format!("Invalid format of key=value pair: '{item}'"));
        }
    }

    Ok((soft_limits, hard_limits))
}

/// Parse the memcheck metrics as error metrics
fn parse_memcheck_metrics(value: &str) -> Result<IndexSet<ErrorMetric>, String> {
    parse_tool_metrics(value, parse_error_metrics)
}

/// Parse --nocapture
fn parse_nocapture(value: &str) -> Result<NoCapture, String> {
    // Taken from clap source code
    const TRUE_LITERALS: [&str; 6] = ["y", "yes", "t", "true", "on", "1"];
    const FALSE_LITERALS: [&str; 6] = ["n", "no", "f", "false", "off", "0"];

    let lowercase = value.to_lowercase();

    if TRUE_LITERALS.contains(&lowercase.as_str()) {
        Ok(NoCapture::True)
    } else if FALSE_LITERALS.contains(&lowercase.as_str()) {
        Ok(NoCapture::False)
    } else if lowercase == "stdout" {
        Ok(NoCapture::Stdout)
    } else if lowercase == "stderr" {
        Ok(NoCapture::Stderr)
    } else {
        Err(format!("Invalid value: {value}"))
    }
}

/// Utility function to parse the --callgrind-metrics, ...
fn parse_tool_metrics<T: Eq + Hash>(
    value: &str,
    parse_metrics: fn(&str) -> Result<IndexSet<T>, String>,
) -> Result<IndexSet<T>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("No metric found: At least one metric must be present".to_owned());
    }

    let mut format = IndexSet::new();

    for item in value.split(',') {
        let item = item.trim();
        let metrics = parse_metrics(item)?;
        format.extend(metrics);
    }

    Ok(format)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::api::EventKind::*;
    use crate::api::RawArgs;

    #[rstest]
    #[case::empty("", &[])]
    #[case::single_key_value("--some=yes", &["--some=yes"])]
    #[case::two_key_value("--some=yes --other=no", &["--some=yes", "--other=no"])]
    #[case::single_escaped("--some='yes and no'", &["--some=yes and no"])]
    #[case::double_escaped("--some='\"yes and no\"'", &["--some=\"yes and no\""])]
    #[case::multiple_escaped("--some='yes and no' --other='no and yes'", &["--some=yes and no", "--other=no and yes"])]
    fn test_parse_callgrind_args(#[case] value: &str, #[case] expected: &[&str]) {
        let actual = parse_args(value).unwrap();
        assert_eq!(actual, RawArgs::from_iter(expected));
    }

    #[rstest]
    #[case::single_soft("Ir=10%", vec![(Ir, 10f64)], vec![])]
    #[case::single_hard("Ir=20", vec![], vec![(Ir, 20.into())])]
    #[case::soft_and_hard("Ir=20|10%", vec![(Ir, 10f64)], vec![(Ir, 20.into())])]
    #[case::soft_and_hard_separated("Ir=20, Ir=10%", vec![(Ir, 10f64)], vec![(Ir, 20.into())])]
    #[case::soft_overwrite("Ir=20%, Ir=10%", vec![(Ir, 10f64)], vec![])]
    #[case::hard_overwrite("Ir=20, Ir=10", vec![], vec![(Ir, 10.into())])]
    #[case::group_wb_soft("@wb=10%", vec![(ILdmr, 10f64), (DLdmr, 10f64), (DLdmw, 10f64)], vec![])]
    #[case::group_writeback_soft(
        "@writeback=10%",
        vec![(ILdmr, 10f64), (DLdmr, 10f64), (DLdmw, 10f64)],
        vec![]
    )]
    #[case::group_writebackbehaviour_soft(
        "@writebackbehaviour=10%",
        vec![(ILdmr, 10f64), (DLdmr, 10f64), (DLdmw, 10f64)],
        vec![]
    )]
    #[case::group_hr_hard_int(
        "@hr=10",
        vec![],
        vec![(L1HitRate, 10f64.into()), (LLHitRate, 10f64.into()), (RamHitRate, 10f64.into())]
    )]
    #[case::group_hr_hard_float(
        "@hr=10.0",
        vec![],
        vec![(L1HitRate, 10f64.into()), (LLHitRate, 10f64.into()), (RamHitRate, 10f64.into())]
    )]
    #[case::case_insensitive(
        "EstIMATedCycles=10%",
        vec![(EstimatedCycles, 10f64)],
        vec![]
    )]
    #[case::multiple_soft(
        "Ir=10%,EstimatedCycles=5%",
        vec![(Ir, 10f64), (EstimatedCycles, 5f64)],
        vec![]
    )]
    #[case::multiple_hard(
        "Ir=20,EstimatedCycles=50",
        vec![],
        vec![(Ir, 20.into()), (EstimatedCycles, 50.into())]
    )]
    #[case::with_whitespace(
        "Ir= 10% , EstimatedCycles = 5%",
        vec![(Ir, 10f64), (EstimatedCycles, 5f64)],
        vec![]
    )]
    fn test_parse_callgrind_limits(
        #[case] regression_var: &str,
        #[case] expected_soft_limits: Vec<(EventKind, f64)>,
        #[case] expected_hard_limits: Vec<(EventKind, Metric)>,
    ) {
        if let ToolRegressionConfig::Callgrind(CallgrindRegressionConfig {
            soft_limits,
            hard_limits,
            ..
        }) = parse_callgrind_limits(regression_var).unwrap()
        {
            assert_eq!(soft_limits, expected_soft_limits);
            assert_eq!(hard_limits, expected_hard_limits);
        } else {
            panic!("Wrong regression config");
        }
    }

    #[rstest]
    #[case::regression_wrong_format_of_key_value_pair(
        "Ir:10",
        "Invalid format of key=value pair: 'Ir:10'"
    )]
    #[case::regression_unknown_event_kind("WRONG=10", "Unknown event kind: 'WRONG'")]
    #[case::float_instead_of_integer(
        "Ir=10.0",
        "Invalid hard limit for 'Instructions': Expected an integer (e.g. '10'). If you wanted \
         this value to be a soft limit use the '%' suffix (e.g. '4.0%' or '4%')"
    )]
    #[case::regression_invalid_percentage(
        "Ir=10.0.0",
        "Invalid hard limit for 'Ir': Invalid metric: invalid float literal"
    )]
    #[case::invalid_soft_limit("Ir=abc%", "Invalid soft limit for 'Ir': invalid float literal")]
    #[case::regression_empty_limits("", "No limits found: At least one limit must be present")]
    fn test_parse_callgrind_limits_then_error(
        #[case] regression_var: &str,
        #[case] expected_reason: &str,
    ) {
        assert_eq!(
            &parse_callgrind_limits(regression_var).unwrap_err(),
            expected_reason,
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_callgrind_args_env() {
        let test_arg = "--just-testing=yes";
        std::env::set_var("IAI_CALLGRIND_CALLGRIND_ARGS", test_arg);
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.callgrind_args,
            Some(RawArgs::new(vec![test_arg.to_owned()]))
        );
    }

    #[test]
    fn test_callgrind_args_not_env() {
        let test_arg = "--just-testing=yes";
        let result = CommandLineArgs::parse_from([format!("--callgrind-args={test_arg}")]);
        assert_eq!(
            result.callgrind_args,
            Some(RawArgs::new(vec![test_arg.to_owned()]))
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_callgrind_args_cli_takes_precedence_over_env() {
        let test_arg_yes = "--just-testing=yes";
        let test_arg_no = "--just-testing=no";
        std::env::set_var("IAI_CALLGRIND_CALLGRIND_ARGS", test_arg_yes);
        let result = CommandLineArgs::parse_from([format!("--callgrind-args={test_arg_no}")]);
        assert_eq!(
            result.callgrind_args,
            Some(RawArgs::new(vec![test_arg_no.to_owned()]))
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_save_summary_env() {
        std::env::set_var("IAI_CALLGRIND_SAVE_SUMMARY", "json");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(result.save_summary, Some(SummaryFormat::Json));
    }

    #[rstest]
    #[case::default("", SummaryFormat::Json)]
    #[case::json("json", SummaryFormat::Json)]
    #[case::pretty_json("pretty-json", SummaryFormat::PrettyJson)]
    fn test_save_summary_cli(#[case] value: &str, #[case] expected: SummaryFormat) {
        let result = if value.is_empty() {
            CommandLineArgs::parse_from(["--save-summary".to_owned()])
        } else {
            CommandLineArgs::parse_from([format!("--save-summary={value}")])
        };
        assert_eq!(result.save_summary, Some(expected));
    }

    #[test]
    #[serial_test::serial]
    fn test_allow_aslr_env() {
        std::env::set_var("IAI_CALLGRIND_ALLOW_ASLR", "yes");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(result.allow_aslr, Some(true));
    }

    #[rstest]
    #[case::default("", true)]
    #[case::yes("yes", true)]
    #[case::no("no", false)]
    fn test_allow_aslr_cli(#[case] value: &str, #[case] expected: bool) {
        let result = if value.is_empty() {
            CommandLineArgs::parse_from(["--allow-aslr".to_owned()])
        } else {
            CommandLineArgs::parse_from([format!("--allow-aslr={value}")])
        };
        assert_eq!(result.allow_aslr, Some(expected));
    }

    #[test]
    #[serial_test::serial]
    fn test_separate_targets_env() {
        std::env::set_var("IAI_CALLGRIND_SEPARATE_TARGETS", "yes");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert!(result.separate_targets);
    }

    #[rstest]
    #[case::default("", true)]
    #[case::yes("yes", true)]
    #[case::no("no", false)]
    fn test_separate_targets_cli(#[case] value: &str, #[case] expected: bool) {
        let result = if value.is_empty() {
            CommandLineArgs::parse_from(["--separate-targets".to_owned()])
        } else {
            CommandLineArgs::parse_from([format!("--separate-targets={value}")])
        };
        assert_eq!(result.separate_targets, expected);
    }

    #[test]
    #[serial_test::serial]
    fn test_home_env() {
        std::env::set_var("IAI_CALLGRIND_HOME", "/tmp/my_iai_home");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(result.home, Some(PathBuf::from("/tmp/my_iai_home")));
    }

    #[test]
    fn test_home_cli() {
        let result = CommandLineArgs::parse_from(["--home=/test_me".to_owned()]);
        assert_eq!(result.home, Some(PathBuf::from("/test_me")));
    }

    #[test]
    fn test_home_cli_when_no_value_then_error() {
        let result = CommandLineArgs::try_parse_from(["--home=".to_owned()]);
        result.unwrap_err();
    }

    #[rstest]
    #[case::default("", NoCapture::True)]
    #[case::yes("true", NoCapture::True)]
    #[case::no("false", NoCapture::False)]
    #[case::stdout("stdout", NoCapture::Stdout)]
    #[case::stderr("stderr", NoCapture::Stderr)]
    fn test_nocapture_cli(#[case] value: &str, #[case] expected: NoCapture) {
        let result = if value.is_empty() {
            CommandLineArgs::parse_from(["--nocapture".to_owned()])
        } else {
            CommandLineArgs::parse_from([format!("--nocapture={value}")])
        };
        assert_eq!(result.nocapture, expected);
    }

    #[test]
    #[serial_test::serial]
    fn test_nocapture_env() {
        std::env::set_var("IAI_CALLGRIND_NOCAPTURE", "true");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(result.nocapture, NoCapture::True);
    }

    #[rstest]
    #[case::single("drd", &[ValgrindTool::DRD])]
    #[case::two("drd,callgrind", &[ValgrindTool::DRD, ValgrindTool::Callgrind])]
    fn test_tools_cli(#[case] tools: &str, #[case] expected: &[ValgrindTool]) {
        let actual = CommandLineArgs::parse_from([format!("--tools={tools}")]);
        assert_eq!(actual.tools, expected);
    }

    #[rstest]
    #[case::y("y", true)]
    #[case::yes("yes", true)]
    #[case::t("t", true)]
    #[case::true_value("true", true)]
    #[case::on("on", true)]
    #[case::one("1", true)]
    #[case::n("n", false)]
    #[case::no("no", false)]
    #[case::f("f", false)]
    #[case::false_value("false", false)]
    #[case::off("off", false)]
    #[case::zero("0", false)]
    fn test_boolish(#[case] value: &str, #[case] expected: bool) {
        let result = CommandLineArgs::parse_from(&[format!("--allow-aslr={value}")]);
        assert_eq!(result.allow_aslr, Some(expected));
    }

    #[rstest]
    #[case::include_ignored("--include-ignored", "")]
    #[case::ignored("--ignored", "")]
    #[case::force_run_in_process("--force-run-in-process", "")]
    #[case::exclude_should_panic("--exclude-should-panic", "")]
    #[case::test("--test", "")]
    #[case::bench("--bench", "")]
    #[case::logfile_without_arg("--logfile", "")]
    #[case::logfile_with_arg("--logfile", "/some/path")]
    #[case::test_threads("--test-threads", "")]
    #[case::skip_without_arg("--skip", "")]
    #[case::skip_with_arg("--skip", "some::test")]
    #[case::quiet_short("-q", "")]
    #[case::quiet_long("--quiet", "")]
    #[case::exact("--exact", "")]
    #[case::color_without_arg("--color", "")]
    #[case::color_with_arg("--color", "auto")]
    #[case::format_without_arg("--format", "")]
    #[case::format_with_arg("--format", "terse")]
    #[case::show_output("--show-output", "")]
    #[case::z_without_arg("-Z", "")]
    #[case::z_with_arg("-Z", "unstable-options")]
    #[case::report_time("--report-time", "")]
    #[case::ensure_time("--ensure-time", "")]
    #[case::shuffle("--shuffle", "")]
    #[case::shuffle_seed_without_arg("--shuffle-seed", "")]
    #[case::shuffle_seed_with_arg("--shuffle-seed", "123")]
    fn test_when_libtest_arg_then_no_exit_with_error(#[case] arg: &str, #[case] value: &str) {
        let result = if value.is_empty() {
            CommandLineArgs::try_parse_from([arg])
        } else {
            CommandLineArgs::try_parse_from(&[format!("{arg}={value}")])
        };

        result.unwrap();
    }

    #[rstest]
    #[case::one("ir", indexset!{ Ir })]
    #[case::one_with_spaces("  ir ", indexset!{ Ir })]
    #[case::two("ir,i1mr", indexset!{ Ir, I1mr })]
    #[case::two_with_spaces("ir,   i1mr", indexset!{ Ir, I1mr })]
    #[case::group("@writebackbehaviour", indexset!{ ILdmr, DLdmr, DLdmw })]
    #[case::group_abbreviation("@wb", indexset!{ ILdmr, DLdmr, DLdmw })]
    #[case::group_and_single_then_no_change("@wb,ildmr", indexset!{ ILdmr, DLdmr, DLdmw })]
    #[case::single_and_group_then_overwrite("dldmw,@wb", indexset!{ DLdmw, ILdmr, DLdmr })]
    #[case::all("@all", CallgrindMetrics::All.into())]
    fn test_parse_callgrind_metrics(#[case] input: &str, #[case] expected: IndexSet<EventKind>) {
        assert_eq!(parse_callgrind_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::empty("")]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    #[case::wrong_delimiter("ir;dr")]
    fn test_parse_callgrind_metrics_then_error(#[case] input: &str) {
        parse_callgrind_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_callgrind_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--callgrind-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_callgrind_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_CALLGRIND_METRICS", "ir");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.callgrind_metrics,
            Some(IndexSet::from([EventKind::Ir]))
        );
    }

    // Just test the very basics. The details are tested in `test_parse_callgrind_metrics`
    #[rstest]
    #[case::one("ir", indexset!{ CachegrindMetric::Ir })]
    #[case::all("@all", CachegrindMetrics::All.into())]
    fn test_parse_cachegrind_metrics(
        #[case] input: &str,
        #[case] expected: IndexSet<CachegrindMetric>,
    ) {
        assert_eq!(parse_cachegrind_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    fn test_parse_cachegrind_metrics_then_error(#[case] input: &str) {
        parse_cachegrind_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_cachegrind_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--cachegrind-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_cachegrind_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_CACHEGRIND_METRICS", "ir");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.cachegrind_metrics,
            Some(IndexSet::from([CachegrindMetric::Ir]))
        );
    }

    #[rstest]
    #[case::one("totalbytes", indexset!{ DhatMetric::TotalBytes })]
    #[case::all("@all", DhatMetrics::All.into())]
    fn test_parse_dhat_metrics(#[case] input: &str, #[case] expected: IndexSet<DhatMetric>) {
        assert_eq!(parse_dhat_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    fn test_parse_dhat_metrics_then_error(#[case] input: &str) {
        parse_dhat_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_dhat_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--dhat-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_dhat_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_DHAT_METRICS", "totalbytes");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.dhat_metrics,
            Some(IndexSet::from([DhatMetric::TotalBytes]))
        );
    }

    #[rstest]
    #[case::one("errors", indexset!{ ErrorMetric::Errors })]
    #[case::all("@all", indexset! {
        ErrorMetric::Errors,
        ErrorMetric::Contexts,
        ErrorMetric::SuppressedErrors,
        ErrorMetric::SuppressedContexts
    })]
    fn test_parse_drd_metrics(#[case] input: &str, #[case] expected: IndexSet<ErrorMetric>) {
        assert_eq!(parse_drd_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    fn test_parse_drd_metrics_then_error(#[case] input: &str) {
        parse_drd_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_drd_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--drd-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_drd_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_DRD_METRICS", "errors");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.drd_metrics,
            Some(IndexSet::from([ErrorMetric::Errors]))
        );
    }

    #[rstest]
    #[case::one("errors", indexset!{ ErrorMetric::Errors })]
    #[case::all("@all", indexset! {
        ErrorMetric::Errors,
        ErrorMetric::Contexts,
        ErrorMetric::SuppressedErrors,
        ErrorMetric::SuppressedContexts
    })]
    fn test_parse_memcheck_metrics(#[case] input: &str, #[case] expected: IndexSet<ErrorMetric>) {
        assert_eq!(parse_memcheck_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    fn test_parse_memcheck_metrics_then_error(#[case] input: &str) {
        parse_memcheck_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_memcheck_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--memcheck-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_memcheck_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_MEMCHECK_METRICS", "errors");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.memcheck_metrics,
            Some(IndexSet::from([ErrorMetric::Errors]))
        );
    }

    #[rstest]
    #[case::one("errors", indexset!{ ErrorMetric::Errors })]
    #[case::all("@all", indexset! {
        ErrorMetric::Errors,
        ErrorMetric::Contexts,
        ErrorMetric::SuppressedErrors,
        ErrorMetric::SuppressedContexts
    })]
    fn test_parse_helgrind_metrics(#[case] input: &str, #[case] expected: IndexSet<ErrorMetric>) {
        assert_eq!(parse_helgrind_metrics(input).unwrap(), expected);
    }

    #[rstest]
    #[case::event_kind_does_not_exist("doesnotexist")]
    #[case::group_does_not_exist("@doesnotexist")]
    fn test_parse_helgrind_metrics_then_error(#[case] input: &str) {
        parse_helgrind_metrics(input).unwrap_err();
    }

    #[test]
    fn test_arg_helgrind_metrics_when_empty_then_error() {
        CommandLineArgs::try_parse_from(["--helgrind-metrics"]).unwrap_err();
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_helgrind_metrics_when_env() {
        std::env::set_var("IAI_CALLGRIND_HELGRIND_METRICS", "errors");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(
            result.helgrind_metrics,
            Some(IndexSet::from([ErrorMetric::Errors]))
        );
    }

    #[rstest]
    #[case::default("--tolerance", f64::from_bits(0.000_01f64.to_bits() - 1))]
    #[case::some_value("--tolerance=1.0", 1.0)]
    fn test_arg_tolerance(#[case] input: &str, #[case] expected: f64) {
        let result = CommandLineArgs::try_parse_from([input]).unwrap();
        assert_eq!(result.tolerance, Some(expected));
    }

    #[test]
    #[serial_test::serial]
    fn test_arg_tolerance_when_env() {
        std::env::set_var("IAI_CALLGRIND_TOLERANCE", "2.0");
        let result = CommandLineArgs::parse_from::<[_; 0], &str>([]);
        assert_eq!(result.tolerance, Some(2.0));
    }
}
