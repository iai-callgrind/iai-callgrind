//! Common structs for `bin_bench` and `lib_bench`

use super::{internal, Direction, EventKind, FlamegraphKind, ValgrindTool};

/// The `FlamegraphConfig` which allows the customization of the created flamegraphs
///
/// Callgrind flamegraphs are very similar to `callgrind_annotate` output. In contrast to
/// `callgrind_annotate` text based output, the produced flamegraphs are svg files (located in the
/// `target/iai` directory) which can be viewed in a browser.
///
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{LibraryBenchmarkConfig, FlamegraphConfig, main};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .flamegraph(FlamegraphConfig::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct FlamegraphConfig(internal::InternalFlamegraphConfig);

/// Configure performance regression checks and behavior
///
/// A performance regression check consists of an [`EventKind`] and a percentage over which a
/// regression is assumed. If the percentage is negative, then a regression is assumed to be below
/// this limit. The default [`EventKind`] is [`EventKind::EstimatedCycles`] with a value of
/// `+10f64`
///
/// If `fail_fast` is set to true, then the whole benchmark run fails on the first encountered
/// regression. Else, the default behavior is, that the benchmark run fails with a regression error
/// after all benchmarks have been run.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{main, LibraryBenchmarkConfig, RegressionConfig};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .regression(RegressionConfig::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Default, Clone)]
pub struct RegressionConfig(internal::InternalRegressionConfig);

/// Configure to run other valgrind tools like `DHAT` or `Massif` in addition to callgrind
///
/// For a list of possible tools see [`ValgrindTool`].
///
/// See also the [Valgrind User Manual](https://valgrind.org/docs/manual/manual.html) for details
/// about possible tools and their command line arguments.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{main, LibraryBenchmarkConfig, Tool, ValgrindTool};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .tool(Tool::new(ValgrindTool::DHAT));
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
pub struct Tool(internal::InternalTool);

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
    /// The default is [`EventKind::EstimatedCycles`]
    ///
    /// Currently, flamegraph creation is limited to one flamegraph for each [`EventKind`] and
    /// there's no way to merge all event kinds into a single flamegraph.
    ///
    /// Note it is an error to specify a [`EventKind`] which isn't recorded by callgrind. See the
    /// docs of the variants of [`EventKind`] which callgrind option is needed to create a record
    /// for it. See also the [Callgrind
    /// Documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options). The
    /// [`EventKind`]s recorded by callgrind which are always available:
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
    /// Additionally, the following derived `EventKinds` are available:
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

impl_traits!(FlamegraphConfig, internal::InternalFlamegraphConfig);

/// Enable performance regression checks with a [`RegressionConfig`]
///
/// A performance regression check consists of an [`EventKind`] and a percentage over which a
/// regression is assumed. If the percentage is negative, then a regression is assumed to be below
/// this limit. The default [`EventKind`] is [`EventKind::EstimatedCycles`] with a value of
/// `+10f64`
///
/// If `fail_fast` is set to true, then the whole benchmark run fails on the first encountered
/// regression. Else, the default behavior is, that the benchmark run fails with a regression error
/// after all benchmarks have been run.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group, main};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// use iai_callgrind::{LibraryBenchmarkConfig, RegressionConfig};
///
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .regression(RegressionConfig::default());
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
impl RegressionConfig {
    /// Configure the limits percentages over/below which a performance regression can be assumed
    ///
    /// A performance regression check consists of an [`EventKind`] and a percentage over which a
    /// regression is assumed. If the percentage is negative, then a regression is assumed to be
    /// below this limit.
    ///
    /// If no `limits` or empty `targets` are specified with this function, the default
    /// [`EventKind`] is [`EventKind::EstimatedCycles`] with a value of `+10f64`
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{EventKind, RegressionConfig};
    ///
    /// let config = RegressionConfig::default().limits([(EventKind::Ir, 5f64)]);
    /// ```
    pub fn limits<T>(&mut self, targets: T) -> &mut Self
    where
        T: IntoIterator<Item = (EventKind, f64)>,
    {
        self.0.limits.extend(targets);
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
    /// use iai_callgrind::RegressionConfig;
    ///
    /// let config = RegressionConfig::default().fail_fast(true);
    /// ```
    pub fn fail_fast(&mut self, value: bool) -> &mut Self {
        self.0.fail_fast = Some(value);
        self
    }
}

impl_traits!(RegressionConfig, internal::InternalRegressionConfig);

impl Tool {
    /// Create a new `Tool` configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Tool, ValgrindTool};
    ///
    /// let tool = Tool::new(ValgrindTool::DHAT);
    /// ```
    pub fn new(tool: ValgrindTool) -> Self {
        Self(internal::InternalTool {
            kind: tool,
            enable: Option::default(),
            outfile_modifier: Option::default(),
            show_log: Option::default(),
            raw_args: internal::InternalRawArgs::default(),
        })
    }

    /// If true, enable running this `Tool` (Default: true)
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Tool, ValgrindTool};
    ///
    /// let tool = Tool::new(ValgrindTool::DHAT).enable(true);
    /// ```
    pub fn enable(&mut self, value: bool) -> &mut Self {
        self.0.enable = Some(value);
        self
    }

    /// Pass one or more arguments directly to the valgrind `Tool`
    ///
    /// Some command line arguments for tools like DHAT (for example `--trace-children=yes`) don't
    /// work without splitting the output into multiple files. Use [`Tool::outfile_modifier`] to
    /// configure splitting the output.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Tool, ValgrindTool};
    ///
    /// let tool = Tool::new(ValgrindTool::DHAT).args(["--num-callers=5", "--mode=heap"]);
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_args.extend_ignore_flag(args);
        self
    }

    /// Add an output and log file modifier like `%p` or `%n`
    ///
    /// The `modifier` is appended to the file name's default extensions `*.out` and `*.log`
    ///
    /// All output file modifiers specified in the [Valgrind
    /// Documentation](https://valgrind.org/docs/manual/manual-core.html#manual-core.options) of
    /// `--log-file` can be used. If using `%q{ENV}` don't forget, that by default all environment
    /// variables are cleared. Either specify to not clear the environment or to
    /// pass-through/define environment variables.
    ///
    /// # Examples
    ///
    /// The following example will result in file names ending with the PID of processes including
    /// their child processes as extension. See also the [Valgrind
    /// Documentation](https://valgrind.org/docs/manual/manual-core.html#manual-core.options) of
    /// `--trace-children` and `--log-file` for more details.
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// use iai_callgrind::{LibraryBenchmarkConfig, main, Tool, ValgrindTool};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// # fn main() {
    ///
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(
    ///             Tool::new(ValgrindTool::DHAT)
    ///                 .args(["--trace-children=yes"])
    ///                 .outfile_modifier("%p")
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn outfile_modifier<T>(&mut self, modifier: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.0.outfile_modifier = Some(modifier.into());
        self
    }
}

impl_traits!(Tool, internal::InternalTool);

/// __DEPRECATED__: A function that is opaque to the optimizer
///
/// It is used to prevent the compiler from optimizing away computations in a benchmark.
///
/// This method is deprecated and is in newer versions of `iai-callgrind` merely a wrapper around
/// [`std::hint::black_box`]. Please use `std::hint::black_box` directly.
#[inline]
pub fn black_box<T>(dummy: T) -> T {
    std::hint::black_box(dummy)
}
