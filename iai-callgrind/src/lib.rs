//! Iai-Callgrind is a benchmarking framework/harness which primarily uses [Valgrind's
//! Callgrind](https://valgrind.org/docs/manual/cl-manual.html) and the other Valgrind tools to
//! provide extremely accurate and consistent measurements of Rust code, making it perfectly suited
//! to run in environments like a CI.
//!
//! # Features
//! - __Precision__: High-precision measurements allow you to reliably detect very small
//! optimizations of your code
//! - __Consistency__: Iai-Callgrind can take accurate measurements even in virtualized CI
//! environments
//! - __Performance__: Since Iai-Callgrind only executes a benchmark once, it is typically a lot
//! faster to run than benchmarks measuring the execution and wall time
//! - __Regression__: Iai-Callgrind reports the difference between benchmark runs to make it easy to
//! spot detailed performance regressions and improvements.
//! - __CPU and Cache Profiling__: Iai-Callgrind generates a Callgrind profile of your code while
//! benchmarking, so you can use Callgrind-compatible tools like
//! [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
//! or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the
//! results in detail.
//! - __Memory Profiling__: You can run other Valgrind tools like [DHAT: a dynamic heap analysis
//! tool](https://valgrind.org/docs/manual/dh-manual.html) and [Massif: a heap
//! profiler](https://valgrind.org/docs/manual/ms-manual.html) with the Iai-Callgrind benchmarking
//! framework. Their profiles are stored next to the callgrind profiles and are ready to be examined
//! with analyzing tools like `dh_view.html`, `ms_print` and others.
//! - __Visualization__: Iai-Callgrind is capable of creating regular and differential flamegraphs
//! from the Callgrind output format.
//! - __Stable-compatible__: Benchmark your code without installing nightly Rust
//!
//! # Benchmarking
//!
//! `iai-callgrind` can be divided into two sections: Benchmarking the library and
//! its public functions and benchmarking of the binaries of a crate.
//!
//! ## Library Benchmarks
//!
//! Use this scheme of the [`main`] macro if you want to benchmark functions of your
//! crate's library.
//!
//! ### Important default behavior
//!
//! The environment variables are cleared before running a library benchmark. See also the
//! Configuration section below if you need to change that behavior.
//!
//! ### Quickstart
//!
//! ```rust
//! use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};
//!
//! // Our function we want to test. Just assume this is a public function in your
//! // library.
//! fn bubble_sort(mut array: Vec<i32>) -> Vec<i32> {
//!     for i in 0..array.len() {
//!         for j in 0..array.len() - i - 1 {
//!             if array[j + 1] < array[j] {
//!                 array.swap(j, j + 1);
//!             }
//!         }
//!     }
//!     array
//! }
//!
//! // This function is used to create a worst case array we want to sort with our
//! // implementation of bubble sort
//! fn setup_worst_case_array(start: i32) -> Vec<i32> {
//!     if start.is_negative() {
//!         (start..0).rev().collect()
//!     } else {
//!         (0..start).rev().collect()
//!     }
//! }
//!
//! // The #[library_benchmark] attribute let's you define a benchmark function which you
//! // can later use in the `library_benchmark_groups!` macro.
//! #[library_benchmark]
//! fn bench_bubble_sort_empty() -> Vec<i32> {
//!     // The `black_box` is needed to tell the compiler to not optimize what's inside
//!     // black_box or else the benchmarks might return inaccurate results.
//!     black_box(bubble_sort(black_box(vec![])))
//! }
//!
//! // This benchmark uses the `bench` attribute to setup benchmarks with different
//! // setups. The big advantage is, that the setup costs and event counts aren't
//! // attributed to the benchmark (and opposed to the old api we don't have to deal with
//! // callgrind arguments, toggles, ...)
//! #[library_benchmark]
//! #[bench::empty(vec![])]
//! #[bench::worst_case_6(vec![6, 5, 4, 3, 2, 1])]
//! // Function calls are fine too
//! #[bench::worst_case_4000(setup_worst_case_array(4000))]
//! // The argument of the benchmark function defines the type of the argument from the
//! // `bench` cases.
//! fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
//!     // Note `array` is not put in a `black_box` because that's already done for you.
//!     black_box(bubble_sort(array))
//! }
//!
//! // A group in which we can put all our benchmark functions
//! library_benchmark_group!(
//!     name = bubble_sort_group;
//!     benchmarks = bench_bubble_sort_empty, bench_bubble_sort
//! );
//!
//! # fn main() {
//! // Finally, the mandatory main! macro which collects all `library_benchmark_groups`.
//! // The main! macro creates a benchmarking harness and runs all the benchmarks defined
//! // in the groups and benches.
//! main!(library_benchmark_groups = bubble_sort_group);
//! # }
//! ```
//!
//! Note that it is important to annotate the benchmark functions with
//! [`#[library_benchmark]`](crate::library_benchmark).
//!
//! ### Configuration
//!
//! It's possible to configure some of the behavior of `iai-callgrind`. See the docs of
//! [`crate::LibraryBenchmarkConfig`] for more details. Configure library benchmarks at
//! top-level with the [`crate::main`] macro, at group level within the
//! [`crate::library_benchmark_group`], at [`crate::library_benchmark`] level
//!
//! and at `bench` level:
//!
//! ```rust
//! # use iai_callgrind::{LibraryBenchmarkConfig, library_benchmark};
//! #[library_benchmark]
//! #[bench::some_id(args = (1, 2), config = LibraryBenchmarkConfig::default())]
//! // ...
//! # fn some_func(first: u8, second: u8) -> u8 {
//! #    first + second
//! # }
//! # fn main() {}
//! ```
//!
//! The config at `bench` level overwrites the config at `library_benchmark` level. The config at
//! `library_benchmark` level overwrites the config at group level and so on. Note that
//! configuration values like `envs` are additive and don't overwrite configuration values of higher
//! levels.
//!
//! See also the docs of [`crate::library_benchmark_group`]. The
//! [README](https://github.com/iai-callgrind/iai-callgrind) of this crate includes more explanations,
//! common recipes and some examples.
//!
//! ## Binary Benchmarks
//!
//! Use this scheme of the [`main`] macro to benchmark one or more binaries of your crate. If you
//! really like to, it's possible to benchmark any executable file in the PATH or any executable
//! specified with an absolute path. The documentation for setting up binary benchmarks with the
//! `binary_benchmark_group` macro can be found in the docs of [`crate::binary_benchmark_group`].
//!
//! ### Temporary Workspace and other important default behavior
//!
//! Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown` functions
//! are executed in a temporary directory. See [`crate::BinaryBenchmarkConfig::sandbox`] for a
//! deeper explanation and how to control and change this behavior. Also, the environment variables
//! of benchmarked binaries are cleared before the benchmark is run. See also
//! [`crate::BinaryBenchmarkConfig::env_clear`] for how to change this behavior.
//!
//! ### Quickstart
//!
//! Suppose your crate's binary is named `my-exe` and you have a fixtures directory in
//! `benches/fixtures` with a file `test1.txt` in it:
//!
//! ```rust
//! use iai_callgrind::{
//!     main, binary_benchmark_group, BinaryBenchmarkConfig, BinaryBenchmarkGroup,
//!     Run, Arg, Fixtures
//! };
//!
//! fn my_setup() {
//!     println!("We can put code in here which will be run before each benchmark run");
//! }
//!
//! // We specify a cmd `"my-exe"` for the whole group which is a binary of our crate. This
//! // eliminates the need to specify a `cmd` for each `Run` later on and we can use the
//! // auto-discovery of a crate's binary at group level. We'll also use the `setup` argument
//! // to run a function before each of the benchmark runs.
//! binary_benchmark_group!(
//!     name = my_exe_group;
//!     setup = my_setup;
//!     // This directory will be copied into the root of the sandbox (as `fixtures`)
//!     config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("benches/fixtures"));
//!     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| setup_my_exe_group(group));
//!
//! // Working within a macro can be tedious sometimes so we moved the setup code into
//! // this method
//! fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
//!     group
//!         // Setup our first run doing something with our fixture `test1.txt`. The
//!         // id (here `do foo with test1`) of an `Arg` has to be unique within the
//!         // same group
//!         .bench(Run::with_arg(Arg::new(
//!             "do foo with test1",
//!             ["--foo=fixtures/test1.txt"],
//!         )))
//!
//!         // Setup our second run with two positional arguments
//!         .bench(Run::with_arg(Arg::new(
//!             "positional arguments",
//!             ["foo", "foo bar"],
//!         )))
//!
//!         // Our last run doesn't take an argument at all.
//!         .bench(Run::with_arg(Arg::empty("no argument")));
//! }
//!
//! # fn main() {
//! // As last step specify all groups we want to benchmark in the main! macro argument
//! // `binary_benchmark_groups`. The main macro is always needed and finally expands
//! // to a benchmarking harness
//! main!(binary_benchmark_groups = my_exe_group);
//! # }
//! ```
//! ### Configuration
//!
//! Much like the configuration of library benchmarks (See above) it's possible to configure binary
//! benchmarks at top-level in the `main!` macro and at group-level in the
//! `binary_benchmark_groups!` with the `config = ...;` argument. In contrast to library benchmarks,
//! binary benchmarks can be configured at a lower and last level within [`Run`] directly.
//!
//! For further details see the section about binary benchmarks of the [`crate::main`] docs the docs
//! of [`crate::binary_benchmark_group`] and [`Run`]. Also, the
//! [README](https://github.com/iai-callgrind/iai-callgrind) of this crate includes some introductory
//! documentation with additional examples.
//!
//! ### Valgrind Tools
//!
//! In addition to the default benchmarks, you can use the Iai-Callgrind framework to run other
//! Valgrind profiling [`Tool`]s like `DHAT`, `Massif` and the experimental `BBV` but also
//! `Memcheck`, `Helgrind` and `DRD` if you need to check memory and thread safety of benchmarked
//! code. See also the [Valgrind User Manual](https://valgrind.org/docs/manual/manual.html) for
//! details and command line arguments. The additional tools can be specified in
//! [`LibraryBenchmarkConfig`], [`BinaryBenchmarkConfig`] or [`Run`]. For example to run `DHAT` for
//! all library benchmarks:
//!
//! ```rust
//! # use iai_callgrind::{library_benchmark, library_benchmark_group};
//! use iai_callgrind::{main, LibraryBenchmarkConfig, Tool, ValgrindTool};
//! # #[library_benchmark]
//! # fn some_func() {}
//! # library_benchmark_group!(name = some_group; benchmarks = some_func);
//! # fn main() {
//! main!(
//!     config = LibraryBenchmarkConfig::default()
//!                 .tool(Tool::new(ValgrindTool::DHAT));
//!     library_benchmark_groups = some_group
//! );
//! # }
//! ```
//!
//! ### Flamegraphs
//!
//! Flamegraphs are opt-in and can be created if you pass a [`FlamegraphConfig`] to the
//! [`BinaryBenchmarkConfig::flamegraph`], [`Run::flamegraph`] or
//! [`LibraryBenchmarkConfig::flamegraph`]. Callgrind flamegraphs are meant as a complement to
//! valgrind's visualization tools `callgrind_annotate` and `kcachegrind`.
//!
//! Callgrind flamegraphs show the inclusive costs for functions and a specific event type, much
//! like `callgrind_annotate` does but in a nicer (and clickable) way. Especially, differential
//! flamegraphs facilitate a deeper understanding of code sections which cause a bottleneck or a
//! performance regressions etc.
//!
//! The produced flamegraph svg files are located next to the respective callgrind output file in
//! the `target/iai` directory.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::default_numeric_fallback)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::fn_to_numeric_cast_any)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::mixed_read_write_in_expression)]
#![warn(clippy::partial_pub_fields)]
#![warn(clippy::rest_pat_in_fully_bound_structs)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_to_string)]
#![warn(clippy::todo)]
#![warn(clippy::try_err)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::unneeded_field_pattern)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::module_name_repetitions)]

macro_rules! impl_traits {
    ($src:ty, $dst:ty) => {
        impl From<$src> for $dst {
            fn from(value: $src) -> Self {
                value.0
            }
        }

        impl From<&$src> for $dst {
            fn from(value: &$src) -> Self {
                value.0.clone()
            }
        }

        impl From<&mut $src> for $dst {
            fn from(value: &mut $src) -> Self {
                value.0.clone()
            }
        }

        impl AsRef<$src> for $src {
            fn as_ref(&self) -> &$src {
                self
            }
        }
    };
}

mod bin_bench;
#[cfg(feature = "client_requests_defs")]
pub mod client_requests;
#[doc(hidden)]
pub mod internal;
mod lib_bench;
mod macros;

pub use bin_bench::{
    Arg, BenchmarkId, BinaryBenchmarkConfig, BinaryBenchmarkGroup, ExitWith, Fixtures, Run,
};
pub use bincode;
pub use iai_callgrind_macros::library_benchmark;
pub use iai_callgrind_runner::api::{Direction, EventKind, FlamegraphKind, ValgrindTool};
pub use lib_bench::LibraryBenchmarkConfig;

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

/// A function that is opaque to the optimizer, used to prevent the compiler from
/// optimizing away computations in a benchmark.
///
/// This variant is stable-compatible, but it may cause some performance overhead
/// or fail to prevent code from being eliminated.
pub fn black_box<T>(dummy: T) -> T {
    // SAFETY: The safety conditions for read_volatile and forget are satisfied
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}
