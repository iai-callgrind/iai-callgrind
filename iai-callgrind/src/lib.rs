//! Iai-Callgrind is a benchmarking framework/harness which uses [Valgrind's
//! Callgrind](https://valgrind.org/docs/manual/cl-manual.html) to provide extremely accurate and
//! consistent measurements of Rust code, making it perfectly suited to run in environments like a
//! CI.
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
//! - __Profiling__: Iai-Callgrind generates a Callgrind profile of your code while benchmarking, so
//! you can use Callgrind-compatible tools like
//! [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
//! or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the
//! results in detail
//! - __Stable-compatible__: Benchmark your code without installing nightly Rust
//!
//! # Benchmarking
//!
//! `iai-callgrind` can be divided into two sections: Benchmarking the library and
//! its public functions and benchmarking of a crate's binary.
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

pub use bincode;
pub use iai_callgrind_macros::library_benchmark;
pub use iai_callgrind_runner::api::{Direction, EventKind, FlamegraphKind, ValgrindTool};

#[doc(hidden)]
pub mod internal;
mod macros;

use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;

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

/// The arguments needed for [`Run`] which are passed to the benchmarked binary
#[derive(Debug, Clone)]
pub struct Arg(internal::InternalArg);

/// An id for an [`Arg`] which can be used to produce unique ids from parameters
#[derive(Debug, Clone)]
pub struct BenchmarkId {
    id: String,
}

/// The main configuration of a binary benchmark.
///
/// Currently it's only possible to pass additional arguments to valgrind's callgrind for all
/// benchmarks. See [`BinaryBenchmarkConfig::raw_callgrind_args`] for more details.
///
/// # Examples
///
/// ```rust,no_run
/// # use iai_callgrind::binary_benchmark_group;
/// # binary_benchmark_group!(name = some_group; benchmark = |group: &mut BinaryBenchmarkGroup| {});
/// use iai_callgrind::{BinaryBenchmarkConfig, main};
///
/// main!(
///     config = BinaryBenchmarkConfig::default().raw_callgrind_args(["toggle-collect=something"]);
///     binary_benchmark_groups = some_group
/// );
/// ```
#[derive(Debug, Default, Clone)]
pub struct BinaryBenchmarkConfig(internal::InternalBinaryBenchmarkConfig);

/// The `BinaryBenchmarkGroup` lets you configure binary benchmark [`Run`]s
#[derive(Debug, Default, Clone)]
pub struct BinaryBenchmarkGroup(internal::InternalBinaryBenchmarkGroup);

/// Set the expected exit status of a binary benchmark
///
/// Per default, the benchmarked binary is expected to succeed, but if a benchmark is expected to
/// fail, setting this option is required.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
/// # binary_benchmark_group!(
/// #    name = my_group;
/// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
/// use iai_callgrind::{main, BinaryBenchmarkConfig, ExitWith};
///
/// # fn main() {
/// main!(
///     config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Code(1));
///     binary_benchmark_groups = my_group
/// );
/// # }
/// ```
#[derive(Debug, Clone)]
pub enum ExitWith {
    /// Exit with success is similar to `ExitCode(0)`
    Success,
    /// Exit with failure is similar to setting the `ExitCode` to something different than `0`
    Failure,
    /// The exact `ExitCode` of the benchmark run
    Code(i32),
}

/// A builder of `Fixtures` to specify the fixtures directory which will be copied into the sandbox
///
/// # Examples
///
/// ```rust
/// use iai_callgrind::Fixtures;
/// let fixtures: Fixtures = Fixtures::new("benches/fixtures");
/// ```
#[derive(Debug, Clone)]
pub struct Fixtures(internal::InternalFixtures);

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

/// The main configuration of a library benchmark.
///
/// See [`LibraryBenchmarkConfig::raw_callgrind_args`] for more details.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{LibraryBenchmarkConfig, main};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .raw_callgrind_args(["toggle-collect=something"]);
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Default)]
pub struct LibraryBenchmarkConfig(internal::InternalLibraryBenchmarkConfig);

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

/// `Run` let's you set up and configure a benchmark run of a binary
#[derive(Debug, Default, Clone)]
pub struct Run(internal::InternalRun);

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

impl Arg {
    /// Create a new `Arg`.
    ///
    /// The `id` must be unique within the same group. It's also possible to use [`BenchmarkId`] as
    /// an argument for the `id` if you want to create unique ids from a parameter.
    ///
    /// An `Arg` can contain multiple arguments which are passed to the benchmarked binary as is. In
    /// the case of no arguments at all, it's more convenient to use [`Arg::empty`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// use std::ffi::OsStr;
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_arg(Arg::new("foo", &["foo"])));
    ///         group.bench(Run::with_arg(Arg::new("foo bar", &["foo", "bar"])));
    ///         group.bench(Run::with_arg(Arg::new("os str foo", &[OsStr::new("foo")])));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    ///
    /// Here's a short example which makes use of the [`BenchmarkId`] to generate unique ids for
    /// each `Arg`:
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, BenchmarkId};
    /// use std::ffi::OsStr;
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         for i in 0..10 {
    ///             group.bench(Run::with_arg(
    ///                 Arg::new(BenchmarkId::new("foo with", i), [format!("--foo={i}")])
    ///             ));
    ///         }
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn new<T, I, U>(id: T, args: U) -> Self
    where
        T: Into<String>,
        I: Into<OsString>,
        U: IntoIterator<Item = I>,
    {
        Self(internal::InternalArg {
            id: Some(id.into()),
            args: args.into_iter().map(std::convert::Into::into).collect(),
        })
    }

    /// Create a new `Arg` with no arguments for the benchmarked binary
    ///
    /// See also [`Arg::new`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_arg(Arg::empty("empty foo")));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn empty<T>(id: T) -> Self
    where
        T: Into<String>,
    {
        Self(internal::InternalArg {
            id: Some(id.into()),
            args: vec![],
        })
    }
}

impl_traits!(Arg, internal::InternalArg);

impl BenchmarkId {
    /// Create a new `BenchmarkId`.
    ///
    /// Use [`BenchmarkId`] as an argument for the `id` of an [`Arg`] if you want to create unique
    /// ids from a parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, BenchmarkId};
    /// use std::ffi::OsStr;
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         for i in 0..10 {
    ///             group.bench(Run::with_arg(
    ///                 Arg::new(BenchmarkId::new("foo with", i), [format!("--foo={i}")])
    ///             ));
    ///         }
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn new<T, P>(id: T, parameter: P) -> Self
    where
        T: AsRef<str>,
        P: Display,
    {
        Self {
            id: format!("{}_{parameter}", id.as_ref()),
        }
    }
}

impl BinaryBenchmarkConfig {
    /// Copy [`Fixtures`] into the sandbox (if enabled)
    ///
    /// See also [`Fixtures`] for details about fixtures and
    /// [`BinaryBenchmarkConfig::sandbox`] for details about the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, Fixtures};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("benches/fixtures"));
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn fixtures<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<internal::InternalFixtures>,
    {
        self.0.fixtures = Some(value.into());
        self
    }

    /// Configure benchmarks to run in a sandbox (Default: true)
    ///
    /// Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown`
    /// functions are executed in a temporary directory. This temporary directory will be created
    /// and changed into before the `before` function is run and removed after the `after` function
    /// has finished. [`BinaryBenchmarkConfig::fixtures`] let's you copy your fixtures into
    /// that directory. If you want to access other directories within the benchmarked package's
    /// directory, you need to specify absolute paths or set the sandbox argument to `false`.
    ///
    /// Another reason for using a temporary directory as workspace is, that the length of the path
    /// where a benchmark is executed may have an influence on the benchmark results. For example,
    /// running the benchmark in your repository `/home/me/my/repository` and someone else's
    /// repository located under `/home/someone/else/repository` may produce different results only
    /// because the length of the first path is shorter. To run benchmarks as deterministic as
    /// possible across different systems, the length of the path should be the same wherever the
    /// benchmark is executed. This crate ensures this property by using the tempfile crate which
    /// creates the temporary directory in `/tmp` with a random name of fixed length like
    /// `/tmp/.tmp12345678`. This ensures that the length of the directory will be the same on all
    /// unix hosts where the benchmarks are run.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().sandbox(false);
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn sandbox(&mut self, value: bool) -> &mut Self {
        self.0.sandbox = Some(value);
        self
    }

    /// Pass arguments to valgrind's callgrind
    ///
    /// It's not needed to pass the arguments with flags. Instead of `--collect-atstart=no` simply
    /// write `collect-atstart=no`.
    ///
    /// See also [Callgrind Command-line
    /// Options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for a full
    /// overview of possible arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::BinaryBenchmarkConfig;
    ///
    /// let config = BinaryBenchmarkConfig::default()
    ///     .raw_callgrind_args(["collect-atstart=no", "toggle-collect=some::path"]);
    /// ```
    pub fn raw_callgrind_args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_callgrind_args.extend_ignore_flag(args);
        self
    }

    /// Add an environment variable
    ///
    /// These environment variables are available independently of the setting of
    /// [`BinaryBenchmarkConfig::env_clear`].
    ///
    /// # Examples
    ///
    /// An example for a custom environment variable "FOO=BAR":
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().env("FOO", "BAR");
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.0.envs.push((key.into(), Some(value.into())));
        self
    }

    /// Add multiple environment variable available in this `Run`
    ///
    /// See also [`Run::env`] for more details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().envs([("FOO", "BAR"),("BAR", "BAZ")]);
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn envs<K, V, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
        T: IntoIterator<Item = (K, V)>,
    {
        self.0
            .envs
            .extend(envs.into_iter().map(|(k, v)| (k.into(), Some(v.into()))));
        self
    }

    /// Specify a pass-through environment variable
    ///
    /// Usually, the environment variables before running a binary benchmark are cleared
    /// but specifying pass-through variables makes this environment variable available to
    /// the benchmark as it actually appeared in the root environment.
    ///
    /// Pass-through environment variables are ignored if they don't exist in the root
    /// environment.
    ///
    /// # Examples
    ///
    /// Here, we chose to pass-through the original value of the `HOME` variable:
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().pass_through_env("HOME");
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn pass_through_env<K>(&mut self, key: K) -> &mut Self
    where
        K: Into<OsString>,
    {
        self.0.envs.push((key.into(), None));
        self
    }

    /// Specify multiple pass-through environment variables
    ///
    /// See also [`LibraryBenchmarkConfig::pass_through_env`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().pass_through_envs(["HOME", "USER"]);
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn pass_through_envs<K, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        T: IntoIterator<Item = K>,
    {
        self.0
            .envs
            .extend(envs.into_iter().map(|k| (k.into(), None)));
        self
    }

    /// If false, don't clear the environment variables before running the benchmark (Default: true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().env_clear(false);
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn env_clear(&mut self, value: bool) -> &mut Self {
        self.0.env_clear = Some(value);
        self
    }

    /// Set the directory of the benchmarked binary (Default: Unchanged)
    ///
    /// Unchanged means, in the case of running with the sandbox enabled, the root of the sandbox.
    /// In the case of running without sandboxing enabled, this'll be the root of the package
    /// directory of the benchmark. If running the benchmark within the sandbox, and the path is
    /// relative then this new directory must be contained in the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().current_dir("/tmp");
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    ///
    /// and the following will change the current directory to `fixtures` assuming it is
    /// contained in the root of the sandbox
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().current_dir("fixtures");
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn current_dir<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<PathBuf>,
    {
        self.0.current_dir = Some(value.into());
        self
    }

    /// Set the start and entry point for event counting of the binary benchmark run
    ///
    /// Per default, the counting of events starts right at the start of the binary and stops when
    /// it finished execution. This'll include some os specific code which makes the executable
    /// actually runnable. To focus on what is actually happening inside the benchmarked binary, it
    /// may desirable to start the counting for example when entering the main function (but can be
    /// any function) and stop counting when leaving the main function of the executable. The
    /// following example will show how to do that.
    ///
    /// # Examples
    ///
    /// The `entry_point` could look like `my_exe::main` for a binary with the name `my-exe` (Note
    /// that hyphens are replaced with an underscore).
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().entry_point("my_exe::main");
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    ///
    /// # About: How to find the right entry point
    ///
    /// If unsure about the entry point, it is best to start without setting the entry point and
    /// inspect the callgrind output file of the benchmark of interest. These are usually located
    /// under `target/iai`. The file format is completely documented
    /// [here](https://valgrind.org/docs/manual/cl-format.html). To focus on the lines of interest
    /// for the entry point, these lines start with `fn=`.
    ///
    /// The example above would include a line which would look like `fn=my_exe::main` with
    /// information about the events below it and maybe some information about the exact location of
    /// this function above it.
    ///
    /// Now, you can set the entry point to what is following the `fn=` entry. To stick to the
    /// example, this would be `my_exe::main`. Running the benchmark again should now show the event
    /// counts of everything happening after entering the main function and before leaving it. If
    /// the counts are `0` (and the main function is not empty), something went wrong and you have
    /// to search the output file again for typos or similar.
    pub fn entry_point<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.0.entry_point = Some(value.into());
        self
    }

    /// Set the expected exit status [`ExitWith`] of a benchmarked binary
    ///
    /// Per default, the benchmarked binary is expected to succeed which is the equivalent of
    /// [`ExitWith::Success`]. But, if a benchmark is expected to fail, setting this option is
    /// required.
    ///
    /// # Examples
    ///
    /// If the benchmark is expected to fail with a specific exit code, for example `100`:
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, ExitWith};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Code(100));
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    ///
    /// If a benchmark is expected to fail, but the exit code doesn't matter:
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, ExitWith};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Failure);
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn exit_with<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<internal::InternalExitWith>,
    {
        self.0.exit_with = Some(value.into());
        self
    }

    /// Option to produce flamegraphs from callgrind output using the [`FlamegraphConfig`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, FlamegraphConfig };
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().flamegraph(FlamegraphConfig::default());
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn flamegraph<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalFlamegraphConfig>,
    {
        self.0.flamegraph = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`RegressionConfig`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, RegressionConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().regression(RegressionConfig::default());
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn regression<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalRegressionConfig>,
    {
        self.0.regression = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`Tool`] in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, Tool, ValgrindTool};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tool(
    ///             Tool::new(ValgrindTool::DHAT)
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tool<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0.tools.update(tool.into());
        self
    }

    /// Add multiple configurations to run valgrind [`Tool`]s in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, Tool, ValgrindTool};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tools(
    ///             [
    ///                 Tool::new(ValgrindTool::DHAT),
    ///                 Tool::new(ValgrindTool::Massif)
    ///             ]
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tools<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0.tools.update_all(tools.into_iter().map(Into::into));
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// See also [`LibraryBenchmarkConfig::tool_override`] for more details.
    ///
    /// # Example
    ///
    /// The following will run `DHAT` and `Massif` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `foo` which will just run `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Run, BinaryBenchmarkConfig, main, Tool, ValgrindTool, Arg
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::new("foo", &["foo"]))
    ///                 .tool_override(Tool::new(ValgrindTool::Memcheck))
    ///         );
    ///     }
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tools(
    ///             [
    ///                 Tool::new(ValgrindTool::DHAT),
    ///                 Tool::new(ValgrindTool::Massif)
    ///             ]
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tool_override<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update(tool.into());
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// See also [`LibraryBenchmarkConfig::tool_override`] for more details.
    ///
    /// # Example
    ///
    /// The following will run `DHAT` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `foo` which will run `Massif` and `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Run, BinaryBenchmarkConfig, main, Tool, ValgrindTool, Arg
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::new("foo", &["foo"]))
    ///                 .tools_override([
    ///                     Tool::new(ValgrindTool::Massif),
    ///                     Tool::new(ValgrindTool::Memcheck),
    ///                 ])
    ///         );
    ///     }
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tool(
    ///             Tool::new(ValgrindTool::DHAT),
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tools_override<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update_all(tools.into_iter().map(Into::into));
        self
    }
}

impl_traits!(
    BinaryBenchmarkConfig,
    internal::InternalBinaryBenchmarkConfig
);

impl BinaryBenchmarkGroup {
    /// Specify a [`Run`] to benchmark a binary
    ///
    /// You can specify a crate's binary either at group level with the simple name of the binary
    /// (say `my-exe`) or at `bench` level with `env!("CARGO_BIN_EXE_my-exe")`. See examples.
    ///
    /// See also [`Run`] for more details.
    ///
    /// # Examples
    ///
    /// If your crate has a binary `my-exe` (the `name` key of a `[[bin]]` entry in Cargo.toml),
    /// specifying `"my-exe"` in the benchmark argument sets the command for all [`Run`]
    /// arguments and it's sufficient to specify only [`Arg`] with [`Run::with_arg`]:
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_arg(Arg::new("foo", &["foo"])));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    ///
    /// Without the `command` at group level:
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |group: &mut BinaryBenchmarkGroup| {
    ///         // Usually you should use `env!("CARGO_BIN_EXE_my-exe")` if `my-exe` is a binary
    ///         // of your crate
    ///         group.bench(Run::with_cmd("/path/to/my-exe", Arg::new("foo", &["foo"])));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    pub fn bench<T>(&mut self, run: T) -> &mut Self
    where
        T: Into<internal::InternalRun>,
    {
        self.0.benches.push(run.into());
        self
    }
}

impl From<internal::InternalBinaryBenchmarkGroup> for BinaryBenchmarkGroup {
    fn from(value: internal::InternalBinaryBenchmarkGroup) -> Self {
        BinaryBenchmarkGroup(value)
    }
}

impl_traits!(BinaryBenchmarkGroup, internal::InternalBinaryBenchmarkGroup);

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

    /// If true, enable running this `Tool`
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

    /// Add a output file modifier like `%p` or `%n`
    ///
    /// The `modifier` is appended to the file name's default extension `*.out`.
    ///
    /// All output file modifiers specified in the [Valgrind
    /// Documentation](https://valgrind.org/docs/manual/manual-core.html#manual-core.options) of
    /// `--log-file` can be used. If using `%q{ENV}` don't forget, that by default all environment
    /// variables are cleared. Either specify to not clear the environment or better specify to
    /// pass-through or define environment variables.
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

impl Fixtures {
    /// Create a new `Fixtures` struct
    ///
    /// The fixtures argument specifies a path to a directory containing fixtures which you want to
    /// be available for all benchmarks and the `before`, `after`, `setup` and `teardown` functions.
    /// Per default, the fixtures directory will be copied as is into the sandbox directory of the
    /// benchmark including symlinks. See [`Fixtures::follow_symlinks`] to change that behavior.
    ///
    /// Relative paths are interpreted relative to the benchmarked package. In a multi-package
    /// workspace this'll be the package name of the benchmark. Otherwise, it'll be the workspace
    /// root.
    ///
    /// # Examples
    ///
    /// Here, the directory `benches/my_fixtures` (with a file `test_1.txt` in it) in the root of
    /// the package under test will be copied into the temporary workspace (for example
    /// `/tmp/.tmp12345678`). So,the benchmarks can access a fixture `test_1.txt` with a relative
    /// path like `my_fixtures/test_1.txt`
    ///
    /// ```rust
    /// use iai_callgrind::Fixtures;
    ///
    /// let fixtures: Fixtures = Fixtures::new("benches/my_fixtures");
    /// ```
    pub fn new<T>(path: T) -> Self
    where
        T: Into<PathBuf>,
    {
        Self(internal::InternalFixtures {
            path: path.into(),
            follow_symlinks: false,
        })
    }

    /// If set to `true`, resolve symlinks in the [`Fixtures`] source directory
    ///
    /// If set to `true` and the [`Fixtures`] directory contains symlinks, these symlinks are
    /// resolved and instead of the symlink the target file or directory will be copied into the
    /// fixtures directory.
    ///
    /// # Examples
    ///
    /// Here, the directory `benches/my_fixtures` (with a symlink `test_1.txt -> ../../test_1.txt`
    /// in it) in the root of the package under test will be copied into the sandbox directory
    /// (for example `/tmp/.tmp12345678`). Since `follow_symlink` is `true`, the benchmarks can
    /// access a fixture `test_1.txt` with a relative path like `my_fixtures/test_1.txt`
    ///
    /// ```rust
    /// use iai_callgrind::Fixtures;
    ///
    /// let fixtures: &mut Fixtures = Fixtures::new("benches/my_fixtures").follow_symlinks(true);
    /// ```
    pub fn follow_symlinks(&mut self, value: bool) -> &mut Self {
        self.0.follow_symlinks = value;
        self
    }
}

impl_traits!(Fixtures, internal::InternalFixtures);

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

impl From<ExitWith> for internal::InternalExitWith {
    fn from(value: ExitWith) -> Self {
        match value {
            ExitWith::Success => Self::Success,
            ExitWith::Failure => Self::Failure,
            ExitWith::Code(c) => Self::Code(c),
        }
    }
}

impl From<&ExitWith> for internal::InternalExitWith {
    fn from(value: &ExitWith) -> Self {
        match value {
            ExitWith::Success => Self::Success,
            ExitWith::Failure => Self::Failure,
            ExitWith::Code(c) => Self::Code(*c),
        }
    }
}

impl LibraryBenchmarkConfig {
    /// Create a new `LibraryBenchmarkConfig` with raw callgrind arguments
    ///
    /// See also [`LibraryBenchmarkConfig::raw_callgrind_args`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// # fn main() {
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// main!(
    ///     config =
    ///         LibraryBenchmarkConfig::with_raw_callgrind_args(["toggle-collect=something"]);
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn with_raw_callgrind_args<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        Self(internal::InternalLibraryBenchmarkConfig {
            env_clear: Option::default(),
            raw_callgrind_args: internal::InternalRawArgs::new(args),
            envs: Vec::default(),
            flamegraph: Option::default(),
            regression: Option::default(),
            tools: internal::InternalTools::default(),
            tools_override: Option::default(),
        })
    }

    /// Add callgrind arguments to this `LibraryBenchmarkConfig`
    ///
    /// The arguments don't need to start with a flag: `--toggle-collect=some` or
    /// `toggle-collect=some` are both understood.
    ///
    /// Not all callgrind arguments are understood by `iai-callgrind` or cause problems in
    /// `iai-callgrind` if they would be applied. `iai-callgrind` will issue a warning in
    /// such cases. Some of the defaults can be overwritten. The default settings are:
    ///
    /// * `--I1=32768,8,64`
    /// * `--D1=32768,8,64`
    /// * `--LL=8388608,16,64`
    /// * `--cache-sim=yes` (can't be changed)
    /// * `--toggle-collect=*BENCHMARK_FILE::BENCHMARK_FUNCTION` (this first toggle can't
    /// be changed)
    /// * `--collect-atstart=no` (overwriting this setting will have no effect)
    /// * `--compress-pos=no`
    /// * `--compress-strings=no`
    ///
    /// Note that `toggle-collect` is an array and the entry point for library benchmarks
    /// is the benchmark function. This default toggle switches event counting on when
    /// entering this benchmark function and off when leaving it. So, additional toggles
    /// for example matching a function within the benchmark function will switch the
    /// event counting off when entering the matched function and on again when leaving
    /// it!
    ///
    /// See also [Callgrind Command-line
    /// Options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// # fn main() {
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///                 .raw_callgrind_args(["toggle-collect=something"]);
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn raw_callgrind_args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.raw_callgrind_args_iter(args);
        self
    }

    /// Add elements of an iterator over callgrind arguments to this `LibraryBenchmarkConfig`
    ///
    /// See also [`LibraryBenchmarkConfig::raw_callgrind_args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///                 .raw_callgrind_args_iter(["toggle-collect=something"].iter());
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn raw_callgrind_args_iter<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.raw_callgrind_args.extend_ignore_flag(args);
        self
    }

    /// Clear the environment variables before running a benchmark (Default: true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().env_clear(false);
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn env_clear(&mut self, value: bool) -> &mut Self {
        self.0.env_clear = Some(value);
        self
    }
    /// Add an environment variables which will be available in library benchmarks
    ///
    /// These environment variables are available independently of the setting of
    /// [`LibraryBenchmarkConfig::env_clear`].
    ///
    /// # Examples
    ///
    /// An example for a custom environment variable, available in all benchmarks:
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().env("FOO", "BAR");
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.0.envs.push((key.into(), Some(value.into())));
        self
    }

    /// Add multiple environment variables which will be available in library benchmarks
    ///
    /// See also [`LibraryBenchmarkConfig::env`] for more details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config =
    ///         LibraryBenchmarkConfig::default().envs([("MY_CUSTOM_VAR", "SOME_VALUE"), ("FOO", "BAR")]);
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn envs<K, V, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
        T: IntoIterator<Item = (K, V)>,
    {
        self.0
            .envs
            .extend(envs.into_iter().map(|(k, v)| (k.into(), Some(v.into()))));
        self
    }

    /// Specify a pass-through environment variable
    ///
    /// Usually, the environment variables before running a library benchmark are cleared
    /// but specifying pass-through variables makes this environment variable available to
    /// the benchmark as it actually appeared in the root environment.
    ///
    /// Pass-through environment variables are ignored if they don't exist in the root
    /// environment.
    ///
    /// # Examples
    ///
    /// Here, we chose to pass-through the original value of the `HOME` variable:
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().pass_through_env("HOME");
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn pass_through_env<K>(&mut self, key: K) -> &mut Self
    where
        K: Into<OsString>,
    {
        self.0.envs.push((key.into(), None));
        self
    }

    /// Specify multiple pass-through environment variables
    ///
    /// See also [`LibraryBenchmarkConfig::pass_through_env`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().pass_through_envs(["HOME", "USER"]);
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn pass_through_envs<K, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        T: IntoIterator<Item = K>,
    {
        self.0
            .envs
            .extend(envs.into_iter().map(|k| (k.into(), None)));
        self
    }

    /// Option to produce flamegraphs from callgrind output using the [`FlamegraphConfig`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main, FlamegraphConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().flamegraph(FlamegraphConfig::default());
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn flamegraph<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalFlamegraphConfig>,
    {
        self.0.flamegraph = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`RegressionConfig`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main, RegressionConfig};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default().regression(RegressionConfig::default());
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn regression<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalRegressionConfig>,
    {
        self.0.regression = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`Tool`] in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main, Tool, ValgrindTool};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(Tool::new(ValgrindTool::DHAT));
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn tool<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0.tools.update(tool.into());
        self
    }

    /// Add multiple configurations to run valgrind [`Tool`]s in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark, library_benchmark_group};
    /// # #[library_benchmark]
    /// # fn some_func() {}
    /// # library_benchmark_group!(name = some_group; benchmarks = some_func);
    /// use iai_callgrind::{LibraryBenchmarkConfig, main, Tool, ValgrindTool};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tools(
    ///             [
    ///                 Tool::new(ValgrindTool::DHAT),
    ///                 Tool::new(ValgrindTool::Massif)
    ///             ]
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn tools<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0.tools.update_all(tools.into_iter().map(Into::into));
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// Usually, if specifying [`Tool`] configurations with [`LibraryBenchmarkConfig::tool`] these
    /// tools are appended to the configuration of a [`LibraryBenchmarkConfig`] of higher-levels.
    /// Specifying a [`Tool`] with this method overrides previously defined configurations.
    ///
    /// Note that [`Tool`]s specified with [`LibraryBenchmarkConfig::tool`] will be ignored, if in
    /// the very same `LibraryBenchmarkConfig`, [`Tool`]s are specified by this method (or
    /// [`LibraryBenchmarkConfig::tools_override`]).
    ///
    /// # Examples
    ///
    /// The following will run `DHAT` and `Massif` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `some_func` which will just run `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     main, library_benchmark, library_benchmark_group, LibraryBenchmarkConfig, Tool, ValgrindTool
    /// };
    ///
    /// #[library_benchmark(config = LibraryBenchmarkConfig::default()
    ///     .tool_override(
    ///         Tool::new(ValgrindTool::Memcheck)
    ///     )
    /// )]
    /// fn some_func() {}
    ///
    /// library_benchmark_group!(
    ///     name = some_group;
    ///     benchmarks = some_func
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tools(
    ///             [
    ///                 Tool::new(ValgrindTool::DHAT),
    ///                 Tool::new(ValgrindTool::Massif)
    ///             ]
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn tool_override<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update(tool.into());
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// See also [`LibraryBenchmarkConfig::tool_override`].
    ///
    /// # Examples
    ///
    /// The following will run `DHAT` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `some_func` which will run `Massif` and `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     main, library_benchmark, library_benchmark_group, LibraryBenchmarkConfig, Tool, ValgrindTool
    /// };
    ///
    /// #[library_benchmark(config = LibraryBenchmarkConfig::default()
    ///     .tools_override([
    ///         Tool::new(ValgrindTool::Massif),
    ///         Tool::new(ValgrindTool::Memcheck)
    ///     ])
    /// )]
    /// fn some_func() {}
    ///
    /// library_benchmark_group!(
    ///     name = some_group;
    ///     benchmarks = some_func
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .tool(
    ///             Tool::new(ValgrindTool::DHAT),
    ///         );
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn tools_override<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update_all(tools.into_iter().map(Into::into));
        self
    }
}

impl_traits!(
    LibraryBenchmarkConfig,
    internal::InternalLibraryBenchmarkConfig
);

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

impl Run {
    /// Create a new `Run` with a `cmd` and [`Arg`]
    ///
    /// A `cmd` specified here overwrites a `cmd` at group level.
    ///
    /// Unlike to a `cmd` specified at group level, there is no auto-discovery of the executables of
    /// a crate, so a crate's binary (say `my-exe`) has to be specified with
    /// `env!("CARGO_BIN_EXE_my-exe")`.
    ///
    /// Although not the main purpose of iai-callgrind, it's possible to benchmark any executable in
    /// the PATH or specified with an absolute path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |group: &mut BinaryBenchmarkGroup| {
    ///         // Usually you should use `env!("CARGO_BIN_EXE_my-exe")` if `my-exe` is a binary
    ///         // of your crate
    ///         group.bench(Run::with_cmd("/path/to/my-exe", Arg::new("foo", &["foo"])));
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn with_cmd<T, U>(cmd: T, arg: U) -> Self
    where
        T: AsRef<str>,
        U: Into<internal::InternalArg>,
    {
        let cmd = cmd.as_ref();
        Self(internal::InternalRun {
            cmd: Some(internal::InternalCmd {
                display: cmd.to_owned(),
                cmd: cmd.to_owned(),
            }),
            args: vec![arg.into()],
            config: internal::InternalBinaryBenchmarkConfig::default(),
        })
    }

    /// Create a new `Run` with a `cmd` and multiple [`Arg`]
    ///
    /// See also [`Run::with_cmd`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_cmd_args("/path/to/my-exe", [
    ///             Arg::empty("empty foo"),
    ///             Arg::new("foo", &["foo"]),
    ///         ]));
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn with_cmd_args<T, I, U>(cmd: T, args: U) -> Self
    where
        T: AsRef<str>,
        I: Into<internal::InternalArg>,
        U: IntoIterator<Item = I>,
    {
        let cmd = cmd.as_ref();

        Self(internal::InternalRun {
            cmd: Some(internal::InternalCmd {
                display: cmd.to_owned(),
                cmd: cmd.to_owned(),
            }),
            args: args.into_iter().map(std::convert::Into::into).collect(),
            config: internal::InternalBinaryBenchmarkConfig::default(),
        })
    }

    /// Create a new `Run` with an [`Arg`]
    ///
    /// If a `cmd` is already specified at group level, there is no need to specify a `cmd` again
    /// (for example with [`Run::with_cmd`]). This method let's you specify a single [`Arg`] to
    /// run with the `cmd` specified at group level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_arg(Arg::new("foo", &["foo"])));
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn with_arg<T>(arg: T) -> Self
    where
        T: Into<internal::InternalArg>,
    {
        Self(internal::InternalRun {
            cmd: None,
            args: vec![arg.into()],
            config: internal::InternalBinaryBenchmarkConfig::default(),
        })
    }

    /// Create a new `Run` with multiple [`Arg`]
    ///
    /// Specifying multiple [`Arg`] arguments is actually just a short-hand for specifying multiple
    /// [`Run`]s with the same configuration and environment variables.
    ///
    /// ```rust
    /// use iai_callgrind::{Arg, BinaryBenchmarkGroup, Run};
    ///
    /// # let mut group1: BinaryBenchmarkGroup = BinaryBenchmarkGroup::default();
    /// # let mut group2: BinaryBenchmarkGroup = BinaryBenchmarkGroup::default();
    /// fn func1(group1: &mut BinaryBenchmarkGroup) {
    ///     group1.bench(Run::with_args([
    ///         Arg::empty("empty foo"),
    ///         Arg::new("foo", &["foo"]),
    ///     ]));
    /// }
    ///
    /// // This is actually the same as above in group1
    /// fn func2(group2: &mut BinaryBenchmarkGroup) {
    ///     group2.bench(Run::with_arg(Arg::empty("empty foo")));
    ///     group2.bench(Run::with_arg(Arg::new("foo", &["foo"])));
    /// }
    /// # func1(&mut group1);
    /// # func2(&mut group2);
    /// ```
    ///
    /// See also [`Run::with_arg`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_args([
    ///             Arg::empty("empty foo"),
    ///             Arg::new("foo", &["foo"])
    ///         ]));
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn with_args<I, T>(args: T) -> Self
    where
        I: Into<internal::InternalArg>,
        T: IntoIterator<Item = I>,
    {
        Self(internal::InternalRun {
            cmd: None,
            args: args.into_iter().map(std::convert::Into::into).collect(),
            config: internal::InternalBinaryBenchmarkConfig::default(),
        })
    }

    /// Add an additional [`Arg`] to the current `Run`
    ///
    /// See also [`Run::with_args`] for more details about a [`Run`] with multiple [`Arg`]
    /// arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .arg(Arg::new("foo", &["foo"]))
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn arg<T>(&mut self, arg: T) -> &mut Self
    where
        T: Into<internal::InternalArg>,
    {
        self.0.args.push(arg.into());
        self
    }

    /// Add multiple additional [`Arg`] arguments to the current `Run`
    ///
    /// See also [`Run::with_args`] for more details about a [`Run`] with multiple [`Arg`]
    /// arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .args([
    ///                     Arg::new("foo", &["foo"]),
    ///                     Arg::new("bar", &["bar"])
    ///             ])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: Into<internal::InternalArg>,
        T: IntoIterator<Item = I>,
    {
        self.0
            .args
            .extend(args.into_iter().map(std::convert::Into::into));
        self
    }

    /// Add an environment variable available in this `Run`
    ///
    /// These environment variables are available independently of the setting of
    /// [`Run::env_clear`].
    ///
    /// # Examples
    ///
    /// An example for a custom environment variable "FOO=BAR":
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo")).env("FOO", "BAR")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.0.config.envs.push((key.into(), Some(value.into())));
        self
    }

    /// Add multiple environment variable available in this `Run`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .envs([("FOO", "BAR"), ("BAR", "BAZ")])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    pub fn envs<K, V, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
        T: IntoIterator<Item = (K, V)>,
    {
        self.0
            .config
            .envs
            .extend(envs.into_iter().map(|(k, v)| (k.into(), Some(v.into()))));
        self
    }

    /// Specify a pass-through environment variable
    ///
    /// See also [`BinaryBenchmarkConfig::pass_through_env`]
    ///
    /// # Examples
    ///
    /// Here, we chose to pass-through the original value of the `HOME` variable:
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .pass_through_env("HOME")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn pass_through_env<K>(&mut self, key: K) -> &mut Self
    where
        K: Into<OsString>,
    {
        self.0.config.envs.push((key.into(), None));
        self
    }

    /// Specify multiple pass-through environment variables
    ///
    /// See also [`BinaryBenchmarkConfig::pass_through_env`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .pass_through_envs(["HOME", "USER"])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn pass_through_envs<K, T>(&mut self, envs: T) -> &mut Self
    where
        K: Into<OsString>,
        T: IntoIterator<Item = K>,
    {
        self.0
            .config
            .envs
            .extend(envs.into_iter().map(|k| (k.into(), None)));
        self
    }

    /// If false, don't clear the environment variables before running the benchmark (Default: true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .env_clear(false)
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn env_clear(&mut self, value: bool) -> &mut Self {
        self.0.config.env_clear = Some(value);
        self
    }

    /// Set the directory of the benchmarked binary (Default: Unchanged)
    ///
    /// See also [`BinaryBenchmarkConfig::current_dir`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .current_dir("/tmp")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// and the following will change the current directory to `fixtures` assuming it is
    /// contained in the root of the sandbox
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, BinaryBenchmarkConfig, Fixtures};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("fixtures"));
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .current_dir("fixtures")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn current_dir<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<PathBuf>,
    {
        self.0.config.current_dir = Some(value.into());
        self
    }

    /// Set the start and entry point for event counting of the binary benchmark run
    ///
    /// See also [`BinaryBenchmarkConfig::entry_point`].
    ///
    /// # Examples
    ///
    /// The `entry_point` could look like `my_exe::main` for a binary with the name `my-exe` (Note
    /// that hyphens are replaced with an underscore).
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .entry_point("my_exe::main")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn entry_point<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.0.config.entry_point = Some(value.into());
        self
    }

    /// Set the expected exit status [`ExitWith`] of a benchmarked binary
    ///
    /// See also [`BinaryBenchmarkConfig::exit_with`]
    ///
    /// # Examples
    ///
    /// If the benchmark is expected to fail with a specific exit code, for example `100`:
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, ExitWith};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .exit_with(ExitWith::Code(100))
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// If a benchmark is expected to fail, but the exit code doesn't matter:
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, ExitWith};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .exit_with(ExitWith::Failure)
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn exit_with<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<internal::InternalExitWith>,
    {
        self.0.config.exit_with = Some(value.into());
        self
    }

    /// Pass arguments to valgrind's callgrind at `Run` level
    ///
    /// See also [`BinaryBenchmarkConfig::raw_callgrind_args`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .raw_callgrind_args(["collect-atstart=no", "toggle-collect=some::path"])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn raw_callgrind_args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.config.raw_callgrind_args.extend_ignore_flag(args);
        self
    }

    /// Option to produce flamegraphs from callgrind output using the [`FlamegraphConfig`]
    ///
    /// See also [`BinaryBenchmarkConfig::flamegraph`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, FlamegraphConfig};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .flamegraph(FlamegraphConfig::default())
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn flamegraph<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalFlamegraphConfig>,
    {
        self.0.config.flamegraph = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`RegressionConfig`]
    ///
    /// See also [`BinaryBenchmarkConfig::regression`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, RegressionConfig};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .regression(RegressionConfig::default())
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn regression<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalRegressionConfig>,
    {
        self.0.config.regression = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`Tool`] in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, Tool, ValgrindTool
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .tool(
    ///                     Tool::new(ValgrindTool::DHAT),
    ///                 )
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn tool<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0.config.tools.update(tool.into());
        self
    }

    /// Add multiple configurations to run valgrind [`Tool`]s in addition to callgrind
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, Tool, ValgrindTool
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .tools([
    ///                     Tool::new(ValgrindTool::DHAT),
    ///                     Tool::new(ValgrindTool::Massif),
    ///                 ])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn tools<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0
            .config
            .tools
            .update_all(tools.into_iter().map(Into::into));
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// See also [`BinaryBenchmarkConfig::tool_override`].
    ///
    /// # Example
    ///
    /// The following will run `DHAT` and `Massif` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `foo` which will just run `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Run, BinaryBenchmarkConfig, main, Tool, ValgrindTool, Arg
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::new("foo", &["foo"]))
    ///                 .tool_override(Tool::new(ValgrindTool::Memcheck))
    ///         );
    ///     }
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tools(
    ///             [
    ///                 Tool::new(ValgrindTool::DHAT),
    ///                 Tool::new(ValgrindTool::Massif)
    ///             ]
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tool_override<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<internal::InternalTool>,
    {
        self.0
            .config
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update(tool.into());
        self
    }

    /// Override previously defined configurations of valgrind [`Tool`]s
    ///
    /// See also [`BinaryBenchmarkConfig::tools_override`].
    ///
    /// # Example
    ///
    /// The following will run `DHAT` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `foo` which will run `Massif` and `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     binary_benchmark_group, Run, BinaryBenchmarkConfig, main, Tool, ValgrindTool, Arg
    /// };
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::new("foo", &["foo"]))
    ///                 .tools_override([
    ///                     Tool::new(ValgrindTool::Massif),
    ///                     Tool::new(ValgrindTool::Memcheck),
    ///                 ])
    ///         );
    ///     }
    /// );
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tool(
    ///             Tool::new(ValgrindTool::DHAT),
    ///         );
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn tools_override<I, T>(&mut self, tools: T) -> &mut Self
    where
        I: Into<internal::InternalTool>,
        T: IntoIterator<Item = I>,
    {
        self.0
            .config
            .tools_override
            .get_or_insert(internal::InternalTools::default())
            .update_all(tools.into_iter().map(Into::into));
        self
    }
}

impl_traits!(Run, internal::InternalRun);

impl From<BenchmarkId> for String {
    fn from(value: BenchmarkId) -> Self {
        value.id
    }
}

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
