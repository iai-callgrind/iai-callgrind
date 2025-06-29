//! Common structs for `bin_bench` and `lib_bench`

use std::vec::Vec;

use derive_more::AsRef;
use iai_callgrind_macros::IntoInner;

use super::{
    CachegrindMetric, CachegrindMetrics, CallgrindMetrics, DhatMetric, Direction, ErrorMetric,
    EventKind, FlamegraphKind, ValgrindTool, __internal,
};
use crate::EntryPoint;

/// The configuration for the experimental bbv
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Bbv};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Bbv::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Bbv(__internal::InternalTool);

/// The configuration for cachegrind
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Cachegrind};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Cachegrind::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Cachegrind(__internal::InternalTool);

/// The configuration for Callgrind
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Callgrind};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Callgrind::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Callgrind(__internal::InternalTool);

/// The configuration for Dhat
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Dhat};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Dhat::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Dhat(__internal::InternalTool);

/// The configuration for DRD
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Drd};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Drd::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Drd(__internal::InternalTool);

/// The `FlamegraphConfig` which allows the customization of the created flamegraphs
///
/// Callgrind flamegraphs are very similar to `callgrind_annotate` output. In contrast to
/// `callgrind_annotate` text based output, the produced flamegraphs are svg files (located in the
/// `target/iai` directory) which can be viewed in a browser.
///
/// # Experimental
///
/// Note the following considerations only affect flamegraphs of multi-threaded/multi-process
/// benchmarks and benchmarks which produce multiple parts with a total over all sub-metrics.
///
/// Currently, Iai-Callgrind creates the flamegraphs only for the total over all threads/parts and
/// subprocesses. This leads to complications since the call graph is not be fully recovered just by
/// examining each thread/subprocess separately. So, the total metrics in the flamegraphs might not
/// be the same as the total metrics shown in the terminal output. If in doubt, the terminal output
/// shows the the correct metrics.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{LibraryBenchmarkConfig, FlamegraphConfig, main, Callgrind};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .tool(Callgrind::default()
///                     .flamegraph(FlamegraphConfig::default())
///                 );
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, Default, IntoInner, AsRef)]
pub struct FlamegraphConfig(__internal::InternalFlamegraphConfig);

/// The configuration for Helgrind
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Helgrind};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Helgrind::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Helgrind(__internal::InternalTool);

/// The configuration for Massif
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Massif};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Massif::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Massif(__internal::InternalTool);

/// The configuration for Memcheck
///
/// Can be specified in [`crate::LibraryBenchmarkConfig::tool`] or
/// [`crate::BinaryBenchmarkConfig::tool`].
///
/// # Example
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Memcheck};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .tool(Memcheck::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Memcheck(__internal::InternalTool);

/// Configure the default output format of the terminal output of Iai-Callgrind
///
/// This configuration is only applied to the default output format (`--output-format=default`) and
/// not to any of the json output formats like (`--output-format=json`).
///
/// # Examples
///
/// For example configure the truncation length of the description to `200` for all library
/// benchmarks in the same file with [`OutputFormat::truncate_description`]:
///
/// ```rust
/// use iai_callgrind::{main, LibraryBenchmarkConfig, OutputFormat};
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(
/// #    name = some_group;
/// #    benchmarks = some_func
/// # );
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///         .output_format(OutputFormat::default()
///             .truncate_description(Some(200))
///         );
///     library_benchmark_groups = some_group
/// );
/// # }
#[derive(Debug, Clone, Default, IntoInner, AsRef)]
pub struct OutputFormat(__internal::InternalOutputFormat);

impl Bbv {
    /// Create a new `BBV` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Bbv::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bbv;
    ///
    /// let config = Bbv::with_args(["interval-size=10000"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(ValgrindTool::BBV, args))
    }

    /// Add command-line arguments to the `BBV` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/bbv-manual.html#bbv-manual.usage> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bbv;
    ///
    /// let config = Bbv::default().args(["interval-size=10000"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// ```rust
    /// use iai_callgrind::Bbv;
    ///
    /// let config = Bbv::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }
}

impl Default for Bbv {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::BBV))
    }
}

impl Cachegrind {
    /// Create a new `Cachegrind` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Cachegrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Cachegrind;
    ///
    /// let config = Cachegrind::with_args(["intr-at-start=no"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::Cachegrind,
            args,
        ))
    }

    /// Add command-line arguments to the `Cachegrind` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Cachegrind;
    ///
    /// let config = Cachegrind::default().args(["intr-at-start=no"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Cachegrind;
    ///
    /// let config = Cachegrind::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Customize the format of the cachegrind output
    ///
    /// See also [`Callgrind::format`] for more details and [`crate::CachegrindMetrics`] for valid
    /// metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{Cachegrind, CachegrindMetric, CachegrindMetrics};
    ///
    /// let config =
    ///     Cachegrind::default().format([CachegrindMetric::Ir.into(), CachegrindMetrics::CacheSim]);
    /// ```
    pub fn format<I, T>(&mut self, cachegrind_metrics: T) -> &mut Self
    where
        I: Into<CachegrindMetrics>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::Cachegrind(Vec::new()));

        if let __internal::InternalToolOutputFormat::Cachegrind(items) = format {
            items.extend(cachegrind_metrics.into_iter().map(Into::into));
        }

        self
    }

    /// Configure the limits percentages over/below which a performance regression can be assumed
    ///
    /// See also [`Callgrind::limits`].
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Cachegrind, CachegrindMetric};
    ///
    /// let config = Cachegrind::default().limits([(CachegrindMetric::Ir, 5f64)]);
    /// ```
    pub fn limits<T>(&mut self, limits: T) -> &mut Self
    where
        T: IntoIterator<Item = (CachegrindMetric, f64)>,
    {
        if let Some(__internal::InternalToolRegressionConfig::Cachegrind(config)) =
            &mut self.0.regression_config
        {
            config.limits.extend(limits);
        } else {
            self.0.regression_config = Some(__internal::InternalToolRegressionConfig::Cachegrind(
                __internal::InternalCachegrindRegressionConfig {
                    limits: limits.into_iter().collect(),
                    fail_fast: None,
                },
            ));
        }
        self
    }

    /// If set to true, then the benchmarks fail on the first encountered regression
    ///
    /// The default is `false` and the whole benchmark run fails with a regression error after all
    /// benchmarks have been run.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::Cachegrind;
    ///
    /// let config = Cachegrind::default().fail_fast(true);
    /// ```
    pub fn fail_fast(&mut self, value: bool) -> &mut Self {
        if let Some(__internal::InternalToolRegressionConfig::Cachegrind(config)) =
            &mut self.0.regression_config
        {
            config.fail_fast = Some(value);
        } else {
            self.0.regression_config = Some(__internal::InternalToolRegressionConfig::Cachegrind(
                __internal::InternalCachegrindRegressionConfig {
                    limits: vec![],
                    fail_fast: Some(value),
                },
            ));
        }
        self
    }
}

impl Default for Cachegrind {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::Cachegrind))
    }
}

impl Callgrind {
    /// Create a new `Callgrind` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Callgrind;
    ///
    /// let config = Callgrind::with_args(["collect-bus=yes"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::Callgrind,
            args,
        ))
    }

    /// Add command-line arguments to the `Callgrind` configuration
    ///
    /// The command-line arguments are passed directly to the callgrind invocation. Valid arguments
    /// are <https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>. Note that not all
    /// command-line arguments are supported especially the ones which change output paths.
    /// Unsupported arguments will be ignored printing a warning.
    ///
    /// The flags can be omitted ("collect-bus" instead of "--collect-bus").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Callgrind;
    ///
    /// let config = Callgrind::default().args(["collect-bus=yes"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// This is mostly useful to disable a tool which has been enabled in a
    /// [`crate::LibraryBenchmarkConfig`] (or [`crate::BinaryBenchmarkConfig`]) at a higher-level.
    /// However, the default tool (usually callgrind) cannot be disabled.
    ///
    /// ```rust
    /// use iai_callgrind::Callgrind;
    ///
    /// let config = Callgrind::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Set or unset the entry point for a benchmark
    ///
    /// Iai-Callgrind sets the [`--toggle-collect`] argument of callgrind to the benchmark function
    /// which we call [`EntryPoint::Default`]. Specifying a `--toggle-collect` argument, sets
    /// automatically `--collect-at-start=no`. This ensures that only the metrics from the benchmark
    /// itself are collected and not the `setup` or `teardown` or anything before/after the
    /// benchmark function.
    ///
    /// However, there are cases when the default toggle is not enough [`EntryPoint::Custom`] or in
    /// the way [`EntryPoint::None`].
    ///
    /// Setting [`EntryPoint::Custom`] is convenience for disabling the entry point with
    /// [`EntryPoint::None`] and setting `--toggle-collect=CUSTOM_ENTRY_POINT` in
    /// [`Callgrind::args`]. [`EntryPoint::Custom`] can be useful if you
    /// want to benchmark a private function and only need the function in the benchmark function as
    /// access point. [`EntryPoint::Custom`] accepts glob patterns the same way as
    /// [`--toggle-collect`] does.
    ///
    /// # Examples
    ///
    /// If you're using callgrind client requests either in the benchmark function itself or in your
    /// library, then using [`EntryPoint::None`] is presumably be required. Consider the following
    /// example (`DEFAULT_ENTRY_POINT` marks the default entry point):
    #[cfg_attr(not(feature = "client_requests_defs"), doc = "```rust,ignore")]
    #[cfg_attr(feature = "client_requests_defs", doc = "```rust")]
    /// use iai_callgrind::{
    ///     main, LibraryBenchmarkConfig,library_benchmark, library_benchmark_group
    /// };
    /// use std::hint::black_box;
    ///
    /// fn to_be_benchmarked() -> u64 {
    ///     println!("Some info output");
    ///     iai_callgrind::client_requests::callgrind::start_instrumentation();
    ///     let result = {
    ///         // some heavy calculations
    /// #       10
    ///     };
    ///     iai_callgrind::client_requests::callgrind::stop_instrumentation();
    ///
    ///     result
    /// }
    ///
    /// #[library_benchmark]
    /// fn some_bench() -> u64 { // <-- DEFAULT ENTRY POINT
    ///     black_box(to_be_benchmarked())
    /// }
    ///
    /// library_benchmark_group!(name = some_group; benchmarks = some_bench);
    /// # fn main() {
    /// main!(library_benchmark_groups = some_group);
    /// # }
    /// ```
    /// 
    /// In the example above [`EntryPoint::Default`] is active, so the counting of events starts
    /// when the `some_bench` function is entered. In `to_be_benchmarked`, the client request
    /// `start_instrumentation` does effectively nothing and `stop_instrumentation` will stop the
    /// event counting as requested. This is most likely not what you intended. The event counting
    /// should start with `start_instrumentation`. To achieve this, you can set [`EntryPoint::None`]
    /// which removes the default toggle, but also `--collect-at-start=no`. So, you need to specify
    /// `--collect-at-start=no` in [`Callgrind::args`]. The example would then look like this:
    /// ```rust
    /// use std::hint::black_box;
    ///
    /// use iai_callgrind::{library_benchmark, EntryPoint, LibraryBenchmarkConfig, Callgrind};
    /// # use iai_callgrind::{library_benchmark_group, main};
    /// # fn to_be_benchmarked() -> u64 { 10 }
    ///
    /// // ...
    ///
    /// #[library_benchmark(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(Callgrind::with_args(["--collect-at-start=no"])
    ///             .entry_point(EntryPoint::None)
    ///         )
    /// )]
    /// fn some_bench() -> u64 {
    ///     black_box(to_be_benchmarked())
    /// }
    ///
    /// // ...
    ///
    /// # library_benchmark_group!(name = some_group; benchmarks = some_bench);
    /// # fn main() {
    /// # main!(library_benchmark_groups = some_group);
    /// # }
    /// ```
    /// [`--toggle-collect`]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options
    pub fn entry_point(&mut self, entry_point: EntryPoint) -> &mut Self {
        self.0.entry_point = Some(entry_point);
        self
    }

    /// Configure the limits percentages over/below which a performance regression can be assumed
    ///
    /// A performance regression check consists of an [`EventKind`] and a percentage over which a
    /// regression is assumed. If the percentage is negative, then a regression is assumed to be
    /// below this limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Callgrind, EventKind};
    ///
    /// let config = Callgrind::default().limits([(EventKind::Ir, 5f64)]);
    /// ```
    pub fn limits<T>(&mut self, limits: T) -> &mut Self
    where
        T: IntoIterator<Item = (EventKind, f64)>,
    {
        if let Some(__internal::InternalToolRegressionConfig::Callgrind(config)) =
            &mut self.0.regression_config
        {
            config.limits.extend(limits);
        } else {
            self.0.regression_config = Some(__internal::InternalToolRegressionConfig::Callgrind(
                __internal::InternalCallgrindRegressionConfig {
                    limits: limits.into_iter().collect(),
                    fail_fast: None,
                },
            ));
        }
        self
    }

    /// If set to true, then the benchmarks fail on the first encountered regression
    ///
    /// The default is `false` and the whole benchmark run fails with a regression error after all
    /// benchmarks have been run.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::Callgrind;
    ///
    /// let config = Callgrind::default().fail_fast(true);
    /// ```
    pub fn fail_fast(&mut self, value: bool) -> &mut Self {
        if let Some(__internal::InternalToolRegressionConfig::Callgrind(config)) =
            &mut self.0.regression_config
        {
            config.fail_fast = Some(value);
        } else {
            self.0.regression_config = Some(__internal::InternalToolRegressionConfig::Callgrind(
                __internal::InternalCallgrindRegressionConfig {
                    limits: vec![],
                    fail_fast: Some(value),
                },
            ));
        }
        self
    }

    /// Option to produce flamegraphs from callgrind output with a [`crate::FlamegraphConfig`]
    ///
    /// The flamegraphs are usable but still in an experimental stage. Callgrind lacks the tool like
    /// `cg_diff` for cachegrind to compare two different profiles. Flamegraphs on the other hand
    /// can bridge the gap and be [`FlamegraphKind::Differential`] to compare two benchmark runs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{
    ///     LibraryBenchmarkConfig, main, FlamegraphConfig, FlamegraphKind, Callgrind
    /// };
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(Callgrind::default()
    ///             .flamegraph(FlamegraphConfig::default()
    ///                 .kind(FlamegraphKind::Differential)
    ///             )
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn flamegraph<T>(&mut self, flamegraph: T) -> &mut Self
    where
        T: Into<__internal::InternalFlamegraphConfig>,
    {
        self.0.flamegraph_config = Some(__internal::InternalToolFlamegraphConfig::Callgrind(
            flamegraph.into(),
        ));
        self
    }

    /// Customize the format of the callgrind output
    ///
    /// This option allows customizing the output format of callgrind metrics. It does not set any
    /// flags for the callgrind execution (i.e. `--branch-sim=yes`) which actually enable the
    /// collection of these metrics. Consult the docs of [`EventKind`] and [`CallgrindMetrics`] to
    /// see which flag is necessary to enable the collection of a specific metric. The rules:
    ///
    /// 1. A metric is only printed if specified here
    /// 2. A metric is not printed if not collected by callgrind
    /// 3. The order matters
    /// 4. In case of duplicate specifications of the same metric the first one wins.
    ///
    /// Callgrind offers a lot of metrics, so the [`CallgrindMetrics`] enum contains groups of
    /// [`EventKind`]s, to avoid having to specify all [`EventKind`]s one-by-one (although still
    /// possible with [`CallgrindMetrics::SingleEvent`]).
    ///
    /// All command-line arguments of callgrind and which metric they collect are described in full
    /// detail in the [callgrind
    /// documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options).
    ///
    /// # Examples
    ///
    /// To enable printing all callgrind metrics specify [`CallgrindMetrics::All`]. `All` callgrind
    /// metrics include the cache misses ([`EventKind::I1mr`], ...). For example in a library
    /// benchmark:
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// use iai_callgrind::{main, LibraryBenchmarkConfig, OutputFormat, CallgrindMetrics, Callgrind};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///                  .tool(Callgrind::default()
    ///                      .format([CallgrindMetrics::All]));
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    ///
    /// The benchmark is executed with the callgrind arguments set by iai-callgrind which don't
    /// collect any other metrics than cache misses (`--cache-sim=yes`), so the output will look
    /// like this:
    ///
    /// ```text
    /// file::some_group::printing cache_misses:
    ///   Instructions:                        1353|1353                 (No change)
    ///   Dr:                                   255|255                  (No change)
    ///   Dw:                                   233|233                  (No change)
    ///   I1mr:                                  54|54                   (No change)
    ///   D1mr:                                  12|12                   (No change)
    ///   D1mw:                                   0|0                    (No change)
    ///   ILmr:                                  53|53                   (No change)
    ///   DLmr:                                   3|3                    (No change)
    ///   DLmw:                                   0|0                    (No change)
    ///   L1 Hits:                             1775|1775                 (No change)
    ///   L2 Hits:                               10|10                   (No change)
    ///   RAM Hits:                              56|56                   (No change)
    ///   Total read+write:                    1841|1841                 (No change)
    ///   Estimated Cycles:                    3785|3785                 (No change)
    /// ```
    pub fn format<I, T>(&mut self, callgrind_metrics: T) -> &mut Self
    where
        I: Into<CallgrindMetrics>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::Callgrind(Vec::new()));

        if let __internal::InternalToolOutputFormat::Callgrind(items) = format {
            items.extend(callgrind_metrics.into_iter().map(Into::into));
        }

        self
    }
}

impl Default for Callgrind {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::Callgrind))
    }
}

impl Dhat {
    /// Create a new `Callgrind` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Dhat::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Dhat;
    ///
    /// let config = Dhat::with_args(["mode=ad-hoc"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::DHAT,
            args,
        ))
    }

    /// Add command-line arguments to the `Dhat` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/dh-manual.html#dh-manual.options> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Dhat;
    ///
    /// let config = Dhat::default().args(["interval-size=10000"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// ```rust
    /// use iai_callgrind::Dhat;
    ///
    /// let config = Dhat::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Customize the format of the dhat output
    ///
    /// See also [`Callgrind::format`] for more details and [`DhatMetric`] for valid metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{Dhat, DhatMetric};
    ///
    /// let config = Dhat::default().format([DhatMetric::TotalBytes, DhatMetric::AtTGmaxBytes]);
    /// ```
    pub fn format<I, T>(&mut self, kinds: T) -> &mut Self
    where
        I: Into<DhatMetric>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::DHAT(Vec::new()));

        if let __internal::InternalToolOutputFormat::DHAT(items) = format {
            items.extend(kinds.into_iter().map(Into::into));
        }

        self
    }
}

impl Default for Dhat {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::DHAT))
    }
}

impl Drd {
    /// Create a new `Drd` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Drd::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Drd;
    ///
    /// let config = Drd::with_args(["exclusive-threshold=100"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(ValgrindTool::DRD, args))
    }

    /// Add command-line arguments to the `Drd` configuration
    ///
    /// Valid arguments are <https://valgrind.org/docs/manual/drd-manual.html#drd-manual.options>
    /// and the core valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Drd;
    ///
    /// let config = Drd::default().args(["exclusive-threshold=100"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Drd;
    ///
    /// let config = Drd::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Customize the format of the `DRD` output
    ///
    /// See also [`Callgrind::format`] for more details and [`ErrorMetric`] for valid metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{Drd, ErrorMetric};
    ///
    /// let config = Drd::default().format([ErrorMetric::Errors, ErrorMetric::SuppressedErrors]);
    /// ```
    pub fn format<I, T>(&mut self, kinds: T) -> &mut Self
    where
        I: Into<ErrorMetric>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::DRD(Vec::new()));

        if let __internal::InternalToolOutputFormat::DRD(items) = format {
            items.extend(kinds.into_iter().map(Into::into));
        }

        self
    }
}

impl Default for Drd {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::DRD))
    }
}

impl FlamegraphConfig {
    /// Option to change the [`FlamegraphKind`]
    ///
    /// The default is [`FlamegraphKind::All`].
    ///
    /// # Examples
    ///
    /// For example, to only create a differential flamegraph:
    ///
    /// ```
    /// use iai_callgrind::{FlamegraphConfig, FlamegraphKind};
    ///
    /// let config = FlamegraphConfig::default().kind(FlamegraphKind::Differential);
    /// ```
    pub fn kind(&mut self, kind: FlamegraphKind) -> &mut Self {
        self.0.kind = Some(kind);
        self
    }

    /// Negate the differential flamegraph [`FlamegraphKind::Differential`]
    ///
    /// The default is `false`.
    ///
    /// Instead of showing the differential flamegraph from the viewing angle of what has happened
    /// the negated differential flamegraph shows what will happen. Especially, this allows to see
    /// vanished event lines (in blue) for example because the underlying code has improved and
    /// removed an unnecessary function call.
    ///
    /// See also [Differential Flame
    /// Graphs](https://www.brendangregg.com/blog/2014-11-09/differential-flame-graphs.html) from
    /// Brendan Gregg's Blog.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{FlamegraphConfig, FlamegraphKind};
    ///
    /// let config = FlamegraphConfig::default().negate_differential(true);
    /// ```
    pub fn negate_differential(&mut self, negate_differential: bool) -> &mut Self {
        self.0.negate_differential = Some(negate_differential);
        self
    }

    /// Normalize the differential flamegraph
    ///
    /// This'll make the first profile event count to match the second. This'll help in situations
    /// when everything looks read (or blue) to get a balanced profile with the full red/blue
    /// spectrum
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{FlamegraphConfig, FlamegraphKind};
    ///
    /// let config = FlamegraphConfig::default().normalize_differential(true);
    /// ```
    pub fn normalize_differential(&mut self, normalize_differential: bool) -> &mut Self {
        self.0.normalize_differential = Some(normalize_differential);
        self
    }

    /// One or multiple [`EventKind`] for which a flamegraph is going to be created.
    ///
    /// The default is [`EventKind::Ir`]
    ///
    /// Currently, flamegraph creation is limited to one flamegraph for each [`EventKind`] and
    /// there's no way to merge all event kinds into a single flamegraph.
    ///
    /// Note it is an error to specify a [`EventKind`] which isn't recorded by callgrind. See the
    /// docs of the variants of [`EventKind`] which callgrind option is needed to create a record
    /// for it. See also the [Callgrind
    /// Documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options). The
    /// [`EventKind`]s recorded by callgrind which are available as long as the cache simulation is
    /// turned on with `--cache-sim=yes` (which is the default):
    ///
    /// * [`EventKind::Ir`]
    /// * [`EventKind::Dr`]
    /// * [`EventKind::Dw`]
    /// * [`EventKind::I1mr`]
    /// * [`EventKind::ILmr`]
    /// * [`EventKind::D1mr`]
    /// * [`EventKind::DLmr`]
    /// * [`EventKind::D1mw`]
    /// * [`EventKind::DLmw`]
    ///
    /// If the cache simulation is turned on, the following derived `EventKinds` are also available:
    ///
    /// * [`EventKind::L1hits`]
    /// * [`EventKind::LLhits`]
    /// * [`EventKind::RamHits`]
    /// * [`EventKind::TotalRW`]
    /// * [`EventKind::EstimatedCycles`]
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{EventKind, FlamegraphConfig};
    ///
    /// let config =
    ///     FlamegraphConfig::default().event_kinds([EventKind::EstimatedCycles, EventKind::Ir]);
    /// ```
    pub fn event_kinds<T>(&mut self, event_kinds: T) -> &mut Self
    where
        T: IntoIterator<Item = EventKind>,
    {
        self.0.event_kinds = Some(event_kinds.into_iter().collect());
        self
    }

    /// Set the [`Direction`] in which the flamegraph should grow.
    ///
    /// The default is [`Direction::TopToBottom`].
    ///
    /// # Examples
    ///
    /// For example to change the default
    ///
    /// ```
    /// use iai_callgrind::{Direction, FlamegraphConfig};
    ///
    /// let config = FlamegraphConfig::default().direction(Direction::BottomToTop);
    /// ```
    pub fn direction(&mut self, direction: Direction) -> &mut Self {
        self.0.direction = Some(direction);
        self
    }

    /// Overwrite the default title of the final flamegraph
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Direction, FlamegraphConfig};
    ///
    /// let config = FlamegraphConfig::default().title("My flamegraph title".to_owned());
    /// ```
    pub fn title(&mut self, title: String) -> &mut Self {
        self.0.title = Some(title);
        self
    }

    /// Overwrite the default subtitle of the final flamegraph
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::FlamegraphConfig;
    ///
    /// let config = FlamegraphConfig::default().subtitle("My flamegraph subtitle".to_owned());
    /// ```
    pub fn subtitle(&mut self, subtitle: String) -> &mut Self {
        self.0.subtitle = Some(subtitle);
        self
    }

    /// Set the minimum width (in pixels) for which event lines are going to be shown.
    ///
    /// The default is `0.1`
    ///
    /// To show all events, set the `min_width` to `0f64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::FlamegraphConfig;
    ///
    /// let config = FlamegraphConfig::default().min_width(0f64);
    /// ```
    pub fn min_width(&mut self, min_width: f64) -> &mut Self {
        self.0.min_width = Some(min_width);
        self
    }
}

impl Helgrind {
    /// Create a new `Helgrind` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Helgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Helgrind;
    ///
    /// let config = Helgrind::with_args(["free-is-write=yes"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::Helgrind,
            args,
        ))
    }

    /// Add command-line arguments to the `Helgrind` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/hg-manual.html#hg-manual.options> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Helgrind;
    ///
    /// let config = Helgrind::default().args(["free-is-write=yes"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Helgrind;
    ///
    /// let config = Helgrind::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Customize the format of the `Helgrind` output
    ///
    /// See also [`Callgrind::format`] for more details and [`ErrorMetric`] for valid metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{ErrorMetric, Helgrind};
    ///
    /// let config = Helgrind::default().format([ErrorMetric::Errors, ErrorMetric::SuppressedErrors]);
    /// ```
    pub fn format<I, T>(&mut self, kinds: T) -> &mut Self
    where
        I: Into<ErrorMetric>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::Helgrind(Vec::new()));

        if let __internal::InternalToolOutputFormat::Helgrind(items) = format {
            items.extend(kinds.into_iter().map(Into::into));
        }

        self
    }
}

impl Default for Helgrind {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::Helgrind))
    }
}

impl Massif {
    /// Create a new `Massif` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Massif::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Massif;
    ///
    /// let config = Massif::with_args(["threshold=2.0"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::Massif,
            args,
        ))
    }

    /// Add command-line arguments to the `Massif` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/ms-manual.html#ms-manual.options> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Massif;
    ///
    /// let config = Massif::default().args(["threshold=2.0"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Massif;
    ///
    /// let config = Massif::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }
}

impl Default for Massif {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::Massif))
    }
}

impl Memcheck {
    /// Create a new `Memcheck` configuration with initial command-line arguments
    ///
    /// See also [`Callgrind::args`] and [`Memcheck::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Memcheck;
    ///
    /// let config = Memcheck::with_args(["free-is-write=yes"]);
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(__internal::InternalTool::with_args(
            ValgrindTool::Memcheck,
            args,
        ))
    }

    /// Add command-line arguments to the `Memcheck` configuration
    ///
    /// Valid arguments
    /// are <https://valgrind.org/docs/manual/mc-manual.html#mc-manual.options> and the core
    /// valgrind command-line arguments
    /// <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>.
    ///
    /// See also [`Callgrind::args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Memcheck;
    ///
    /// let config = Memcheck::default().args(["show-leak-kinds=all"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Enable this tool. This is the default.
    ///
    /// See also [`Callgrind::enable`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Memcheck;
    ///
    /// let config = Memcheck::default().enable(false);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Customize the format of the `Memcheck` output
    ///
    /// See also [`Callgrind::format`] for more details and [`ErrorMetric`] for valid metrics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{ErrorMetric, Memcheck};
    ///
    /// let config = Memcheck::default().format([ErrorMetric::Errors, ErrorMetric::SuppressedErrors]);
    /// ```
    pub fn format<I, T>(&mut self, kinds: T) -> &mut Self
    where
        I: Into<ErrorMetric>,
        T: IntoIterator<Item = I>,
    {
        let format = self
            .0
            .output_format
            .get_or_insert_with(|| __internal::InternalToolOutputFormat::Memcheck(Vec::new()));

        if let __internal::InternalToolOutputFormat::Memcheck(items) = format {
            items.extend(kinds.into_iter().map(Into::into));
        }

        self
    }
}

impl Default for Memcheck {
    fn default() -> Self {
        Self(__internal::InternalTool::new(ValgrindTool::Memcheck))
    }
}

impl OutputFormat {
    /// Adjust, enable or disable the truncation of the description in the iai-callgrind output
    ///
    /// The default is to truncate the description to the size of 50 ascii characters. A `None`
    /// value disables the truncation entirely and a `Some` value will truncate the description to
    /// the given amount of characters excluding the ellipsis.
    ///
    /// To clearify which part of the output is meant by `DESCRIPTION`:
    ///
    /// ```text
    /// benchmark_file::group_name::function_name id:DESCRIPTION
    ///   Instructions:              352135|352135          (No change)
    ///   L1 Hits:                   470117|470117          (No change)
    ///   L2 Hits:                      748|748             (No change)
    ///   RAM Hits:                    4112|4112            (No change)
    ///   Total read+write:          474977|474977          (No change)
    ///   Estimated Cycles:          617777|617777          (No change)
    /// ```
    ///
    /// # Examples
    ///
    /// For example, specifying this option with a `None` value in the `main!` macro disables the
    /// truncation of the description for all benchmarks.
    ///
    /// ```rust
    /// use iai_callgrind::{main, LibraryBenchmarkConfig, OutputFormat};
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(
    /// #    name = some_group;
    /// #    benchmarks = some_func
    /// # );
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .output_format(OutputFormat::default()
    ///             .truncate_description(None)
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn truncate_description(&mut self, value: Option<usize>) -> &mut Self {
        self.0.truncate_description = Some(value);
        self
    }

    /// Show intermediate metrics from parts, subprocesses, threads, ... (Default: false)
    ///
    /// In callgrind, threads are treated as separate units (similar to subprocesses) and the
    /// metrics for them are dumped into an own file. Other valgrind tools usually separate the
    /// output files only by subprocesses. To also show the metrics of any intermediate fragments
    /// and not just the total over all of them, set the value of this method to `true`.
    ///
    /// Temporarily setting `show_intermediate` to `true` can help to find misconfigurations in
    /// multi-thread/multi-process benchmarks.
    ///
    /// # Examples
    ///
    /// As opposed to valgrind/callgrind, `--trace-children=yes`, `--separate-threads=yes` and
    /// `--fair-sched=try` are the defaults in Iai-Callgrind, so in the following example it's not
    /// necessary to specify `--separate-threads` to track the metrics of the spawned thread.
    /// However, it is necessary to specify an additional toggle or else the metrics of the thread
    /// are all zero. We also set the [`super::EntryPoint`] to `None` to disable the default entry
    /// point (toggle) which is the benchmark function. So, with this setup we collect only the
    /// metrics of the method `my_lib::heavy_calculation` in the spawned thread and nothing else.
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     main, LibraryBenchmarkConfig, OutputFormat, EntryPoint, library_benchmark,
    ///     library_benchmark_group, Callgrind
    /// };
    /// # mod my_lib { pub fn heavy_calculation() -> u64 { 42 }}
    ///
    /// #[library_benchmark(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(Callgrind::with_args(["--toggle-collect=my_lib::heavy_calculation"])
    ///             .entry_point(EntryPoint::None)
    ///         )
    ///         .output_format(OutputFormat::default().show_intermediate(true))
    /// )]
    /// fn bench_thread() -> u64 {
    ///     let handle = std::thread::spawn(|| my_lib::heavy_calculation());
    ///     handle.join().unwrap()
    /// }
    ///
    /// library_benchmark_group!(name = some_group; benchmarks = bench_thread);
    /// # fn main() {
    /// main!(library_benchmark_groups = some_group);
    /// # }
    /// ```
    ///
    /// Running the above benchmark the first time will print something like the below (The exact
    /// metric counts are made up for demonstration purposes):
    ///
    /// ```text
    /// my_benchmark::some_group::bench_thread
    ///   ## pid: 633247 part: 1 thread: 1   |N/A
    ///   Command:            target/release/deps/my_benchmark-08fe8356975cd1af
    ///   Instructions:                     0|N/A             (*********)
    ///   L1 Hits:                          0|N/A             (*********)
    ///   L2 Hits:                          0|N/A             (*********)
    ///   RAM Hits:                         0|N/A             (*********)
    ///   Total read+write:                 0|N/A             (*********)
    ///   Estimated Cycles:                 0|N/A             (*********)
    ///   ## pid: 633247 part: 1 thread: 2   |N/A
    ///   Command:            target/release/deps/my_benchmark-08fe8356975cd1af
    ///   Instructions:                  3905|N/A             (*********)
    ///   L1 Hits:                       4992|N/A             (*********)
    ///   L2 Hits:                          0|N/A             (*********)
    ///   RAM Hits:                       464|N/A             (*********)
    ///   Total read+write:              5456|N/A             (*********)
    ///   Estimated Cycles:             21232|N/A             (*********)
    ///   ## Total
    ///   Instructions:                  3905|N/A             (*********)
    ///   L1 Hits:                       4992|N/A             (*********)
    ///   L2 Hits:                          0|N/A             (*********)
    ///   RAM Hits:                       464|N/A             (*********)
    ///   Total read+write:              5456|N/A             (*********)
    ///   Estimated Cycles:             21232|N/A             (*********)
    /// ```
    ///
    /// With `show_intermediate` set to `false` (the default), only the total is shown:
    ///
    /// ```text
    /// my_benchmark::some_group::bench_thread
    ///   Instructions:                  3905|N/A             (*********)
    ///   L1 Hits:                       4992|N/A             (*********)
    ///   L2 Hits:                          0|N/A             (*********)
    ///   RAM Hits:                       464|N/A             (*********)
    ///   Total read+write:              5456|N/A             (*********)
    ///   Estimated Cycles:             21232|N/A             (*********)
    /// ```
    pub fn show_intermediate(&mut self, value: bool) -> &mut Self {
        self.0.show_intermediate = Some(value);
        self
    }

    /// Show an ascii grid in the benchmark terminal output
    ///
    /// This option adds guiding lines which can help reading the benchmark output when running
    /// multiple tools with multiple threads/subprocesses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::OutputFormat;
    ///
    /// let output_format = OutputFormat::default().show_grid(true);
    /// ```
    ///
    /// Below is the output of a Iai-Callgrind run with DHAT as additional tool benchmarking a
    /// function that executes a subprocess which itself starts multiple threads. For the benchmark
    /// run below [`OutputFormat::show_intermediate`] was also active to show the threads and
    /// subprocesses.
    ///
    /// ```text
    /// test_lib_bench_threads::bench_group::bench_thread_in_subprocess three:3
    /// |======== CALLGRIND ===================================================================
    /// |-## pid: 3186352 part: 1 thread: 1       |pid: 2721318 part: 1 thread: 1
    /// | Command:            target/release/deps/test_lib_bench_threads-b0b85adec9a45de1
    /// | Instructions:                       4697|4697                 (No change)
    /// | L1 Hits:                            6420|6420                 (No change)
    /// | L2 Hits:                              17|17                   (No change)
    /// | RAM Hits:                            202|202                  (No change)
    /// | Total read+write:                   6639|6639                 (No change)
    /// | Estimated Cycles:                  13575|13575                (No change)
    /// |-## pid: 3186468 part: 1 thread: 1       |pid: 2721319 part: 1 thread: 1
    /// | Command:            target/release/thread 3
    /// | Instructions:                      35452|35452                (No change)
    /// | L1 Hits:                           77367|77367                (No change)
    /// | L2 Hits:                             610|610                  (No change)
    /// | RAM Hits:                            784|784                  (No change)
    /// | Total read+write:                  78761|78761                (No change)
    /// | Estimated Cycles:                 107857|107857               (No change)
    /// |-## pid: 3186468 part: 1 thread: 2       |pid: 2721319 part: 1 thread: 2
    /// | Command:            target/release/thread 3
    /// | Instructions:                    2460507|2460507              (No change)
    /// | L1 Hits:                         2534939|2534939              (No change)
    /// | L2 Hits:                              17|17                   (No change)
    /// | RAM Hits:                            186|186                  (No change)
    /// | Total read+write:                2535142|2535142              (No change)
    /// | Estimated Cycles:                2541534|2541534              (No change)
    /// |-## pid: 3186468 part: 1 thread: 3       |pid: 2721319 part: 1 thread: 3
    /// | Command:            target/release/thread 3
    /// | Instructions:                    3650414|3650414              (No change)
    /// | L1 Hits:                         3724275|3724275              (No change)
    /// | L2 Hits:                              21|21                   (No change)
    /// | RAM Hits:                            130|130                  (No change)
    /// | Total read+write:                3724426|3724426              (No change)
    /// | Estimated Cycles:                3728930|3728930              (No change)
    /// |-## pid: 3186468 part: 1 thread: 4       |pid: 2721319 part: 1 thread: 4
    /// | Command:            target/release/thread 3
    /// | Instructions:                    4349846|4349846              (No change)
    /// | L1 Hits:                         4423438|4423438              (No change)
    /// | L2 Hits:                              24|24                   (No change)
    /// | RAM Hits:                            125|125                  (No change)
    /// | Total read+write:                4423587|4423587              (No change)
    /// | Estimated Cycles:                4427933|4427933              (No change)
    /// |-## Total
    /// | Instructions:                   10500916|10500916             (No change)
    /// | L1 Hits:                        10766439|10766439             (No change)
    /// | L2 Hits:                             689|689                  (No change)
    /// | RAM Hits:                           1427|1427                 (No change)
    /// | Total read+write:               10768555|10768555             (No change)
    /// | Estimated Cycles:               10819829|10819829             (No change)
    /// |======== DHAT ========================================================================
    /// |-## pid: 3186472 ppid: 3185288           |pid: 2721323 ppid: 2720196
    /// | Command:            target/release/deps/test_lib_bench_threads-b0b85adec9a45de1
    /// | Total bytes:                        2774|2774                 (No change)
    /// | Total blocks:                         24|24                   (No change)
    /// | At t-gmax bytes:                    1736|1736                 (No change)
    /// | At t-gmax blocks:                      3|3                    (No change)
    /// | At t-end bytes:                        0|0                    (No change)
    /// | At t-end blocks:                       0|0                    (No change)
    /// | Reads bytes:                       21054|21054                (No change)
    /// | Writes bytes:                      13165|13165                (No change)
    /// |-## pid: 3186473 ppid: 3186472           |pid: 2721324 ppid: 2721323
    /// | Command:            target/release/thread 3
    /// | Total bytes:                      156158|156158               (No change)
    /// | Total blocks:                         73|73                   (No change)
    /// | At t-gmax bytes:                   52225|52225                (No change)
    /// | At t-gmax blocks:                     19|19                   (No change)
    /// | At t-end bytes:                        0|0                    (No change)
    /// | At t-end blocks:                       0|0                    (No change)
    /// | Reads bytes:                      118403|118403               (No change)
    /// | Writes bytes:                     135926|135926               (No change)
    /// |-## Total
    /// | Total bytes:                      158932|158932               (No change)
    /// | Total blocks:                         97|97                   (No change)
    /// | At t-gmax bytes:                   53961|53961                (No change)
    /// | At t-gmax blocks:                     22|22                   (No change)
    /// | At t-end bytes:                        0|0                    (No change)
    /// | At t-end blocks:                       0|0                    (No change)
    /// | Reads bytes:                      139457|139457               (No change)
    /// | Writes bytes:                     149091|149091               (No change)
    /// |-Comparison with bench_find_primes_multi_thread three:3
    /// | Instructions:                   10494117|10500916             (-0.06475%) [-1.00065x]
    /// | L1 Hits:                        10757259|10766439             (-0.08526%) [-1.00085x]
    /// | L2 Hits:                             601|689                  (-12.7721%) [-1.14642x]
    /// | RAM Hits:                           1189|1427                 (-16.6783%) [-1.20017x]
    /// | Total read+write:               10759049|10768555             (-0.08828%) [-1.00088x]
    /// | Estimated Cycles:               10801879|10819829             (-0.16590%) [-1.00166x]
    pub fn show_grid(&mut self, value: bool) -> &mut Self {
        self.0.show_grid = Some(value);
        self
    }
}
