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
//! Use this scheme of the [`main`] macro if you want to benchmark functions of your crate's
//! library.
//!
//! ### Quickstart
//!
//! ```rust
//! use iai_callgrind::{black_box, main};
//!
//! // Assume this function would be a public function of your library
//! fn fibonacci(n: u64) -> u64 {
//!     match n {
//!         0 => 1,
//!         1 => 1,
//!         n => fibonacci(n - 1) + fibonacci(n - 2),
//!     }
//! }
//!
//! #[inline(never)] // required for benchmarking functions
//! fn iai_benchmark_short() -> u64 {
//!     fibonacci(black_box(10))
//! }
//!
//! #[inline(never)] // required for benchmarking functions
//! fn iai_benchmark_long() -> u64 {
//!     fibonacci(black_box(30))
//! }
//!
//! # fn main() {
//! main!(iai_benchmark_short, iai_benchmark_long);
//! # }
//! ```
//!
//! Note that it is important to annotate the benchmark functions with `#[inline(never)]` or else
//! the rust compiler will most likely try to optimize this function and inline it.
//!
//! The [README](https://github.com/Joining7943/iai-callgrind) of this crate includes more
//! explanations, common recipes and some additional examples.
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
//! are executed in a temporary directory. See [`BinaryBenchmarkGroup::sandbox`] for a deeper
//! explanation and how to control and change this behavior. Also, the environment variables of
//! benchmarked binaries are cleared before the benchmark is run. See also [`Options::env_clear`]
//! for how to change this behavior.
//!
//! ### Quickstart
//!
//! Suppose your crate's binary is named `my-exe` and you have a fixtures directory in
//! `benches/fixtures` with a file `test1.txt` in it:
//!
//! ```rust
//! use iai_callgrind::{main, binary_benchmark_group, BinaryBenchmarkGroup, Run, Arg, Fixtures};
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
//!     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| setup_my_exe_group(group));
//!
//! // Working within a macro can be tedious sometimes so we moved the setup code into
//! // this method
//! fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
//!     group
//!         // This directory will be copied into the root of the sandbox (as `fixtures`)
//!         .fixtures(Fixtures::new("benches/fixtures"))
//!
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
//!
//! For further details see the section about binary benchmarks of the [`crate::main`] docs and the
//! docs of [`crate::binary_benchmark_group`]. Also, the
//! [README](https://github.com/Joining7943/iai-callgrind) of this crate includes some introductory
//! documentation with additional examples.

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

pub mod internal;
mod macros;

use std::ffi::OsStr;
use std::fmt::Display;
use std::path::PathBuf;

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

/// TODO: DOCUMENT
#[derive(Debug, Default)]
pub struct LibraryBenchmarkConfig(internal::RunnerLibraryBenchmarkConfig);

impl LibraryBenchmarkConfig {
    /// TODO: DOCUMENT
    pub fn with_raw_callgrind_args<I: AsRef<str>, T: AsRef<[I]>>(args: T) -> Self {
        Self(internal::RunnerLibraryBenchmarkConfig {
            raw_callgrind_args: internal::RunnerRawCallgrindArgs::new(args),
        })
    }

    /// TODO: DOCUMENT
    pub fn raw_callgrind_args<I: AsRef<str>, T: AsRef<[I]>>(&mut self, args: T) -> &mut Self {
        self.raw_callgrind_args_iter(args.as_ref().iter());
        self
    }

    /// TODO: DOCUMENT
    pub fn raw_callgrind_args_iter<I: AsRef<str>, T: Iterator<Item = I>>(
        &mut self,
        args: T,
    ) -> &mut Self {
        self.0.raw_callgrind_args.raw_callgrind_args_iter(args);
        self
    }
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
/// use iai_callgrind::BinaryBenchmarkConfig;
///
/// iai_callgrind::main!(
///     config = BinaryBenchmarkConfig::default().raw_callgrind_args(["toggle-collect=something"]);
///     binary_benchmark_groups = some_group
/// );
/// ```
#[derive(Debug, Default, Clone)]
pub struct BinaryBenchmarkConfig(internal::RunnerConfig);

impl BinaryBenchmarkConfig {
    /// Pass arguments to valgrind's callgrind for all benchmarks within the same file
    ///
    /// It's not needed to pass the arguments with flags. Instead of `--collect-atstart=no` simply
    /// write `collect-atstart=no`.
    ///
    /// It's possible to overwrite some of the defaults which currently are:
    /// * --I1=32768,8,64
    /// * --D1=32768,8,64
    /// * --LL=8388608,16,64
    /// * --cache-sim=yes (can't be changed)
    /// * --collect-atstart=yes
    /// * --compress-pos=no (not recommended)
    /// * --compress-strings=no (not recommended)
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
    pub fn raw_callgrind_args<I: AsRef<str>, T: AsRef<[I]>>(&mut self, args: T) -> &mut Self {
        self.0
            .raw_callgrind_args
            .raw_callgrind_args_iter(args.as_ref().iter());
        self
    }
}

/// The `BinaryBenchmarkGroup` lets you configure and execute benchmarks
#[derive(Debug, Default, Clone)]
pub struct BinaryBenchmarkGroup(internal::RunnerBinaryBenchmarkGroup);

impl BinaryBenchmarkGroup {
    /// Copy [`Fixtures`] into the sandbox (if enabled)
    ///
    /// See also [`Fixtures`] for details about fixtures and [`BinaryBenchmarkGroup::sandbox`] for
    /// details about the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{BinaryBenchmarkGroup, Fixtures};
    ///
    /// # let mut group: BinaryBenchmarkGroup = BinaryBenchmarkGroup::default();
    /// fn func(group: &mut BinaryBenchmarkGroup) {
    ///     group.fixtures(Fixtures::new("benches/fixtures"));
    /// }
    /// # func(&mut group);
    /// ```
    pub fn fixtures<T: Into<internal::RunnerFixtures>>(&mut self, value: T) -> &mut Self {
        self.0.fixtures = Some(value.into());
        self
    }

    /// Configure benchmarks to run in a sandbox (Default: true)
    ///
    /// Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown`
    /// functions are executed in a temporary directory. This temporary directory will be created
    /// and changed into before the `before` function is run and removed after the `after` function
    /// has finished. [`BinaryBenchmarkGroup::fixtures`] let's you copy your fixtures into that
    /// directory. If you want to access other directories within the benchmarked package's
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
    /// use iai_callgrind::BinaryBenchmarkGroup;
    ///
    /// # let mut group: BinaryBenchmarkGroup = BinaryBenchmarkGroup::default();
    /// fn func(group: &mut BinaryBenchmarkGroup) {
    ///     group.sandbox(false);
    /// }
    /// # func(&mut group);
    /// ```
    pub fn sandbox(&mut self, value: bool) -> &mut Self {
        self.0.sandbox = value;
        self
    }

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
    pub fn bench<T: Into<internal::RunnerRun>>(&mut self, run: T) -> &mut Self {
        self.0.benches.push(run.into());
        self
    }
}

impl From<internal::RunnerBinaryBenchmarkGroup> for BinaryBenchmarkGroup {
    fn from(value: internal::RunnerBinaryBenchmarkGroup) -> Self {
        BinaryBenchmarkGroup(value)
    }
}

/// `Run` let's you set up and configure a benchmark run of a binary
#[derive(Debug, Default, Clone)]
pub struct Run(internal::RunnerRun);

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
    pub fn with_cmd<T: AsRef<str>, U: Into<internal::RunnerArg>>(cmd: T, arg: U) -> Self {
        let cmd = cmd.as_ref();
        Self(internal::RunnerRun {
            cmd: Some(internal::RunnerCmd {
                display: cmd.to_owned(),
                cmd: cmd.to_owned(),
            }),
            args: vec![arg.into()],
            opts: None,
            envs: Vec::default(),
        })
    }

    /// Create a new `Run` with a `cmd` and multiple [`Arg`]
    ///
    /// See also [`Run::with_cmd`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_cmd_args("/path/to/my-exe", [
    ///             Arg::empty("empty foo"),
    ///             Arg::new("foo", &["foo"]),
    ///         ]));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn with_cmd_args<T: AsRef<str>, U: AsRef<[Arg]>>(cmd: T, args: U) -> Self {
        let cmd = cmd.as_ref();
        let args = args.as_ref();

        Self(internal::RunnerRun {
            cmd: Some(internal::RunnerCmd {
                display: cmd.to_owned(),
                cmd: cmd.to_owned(),
            }),
            args: args.iter().map(std::convert::Into::into).collect(),
            opts: None,
            envs: Vec::default(),
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
    pub fn with_arg<T: Into<internal::RunnerArg>>(arg: T) -> Self {
        Self(internal::RunnerRun {
            cmd: None,
            args: vec![arg.into()],
            opts: None,
            envs: Vec::default(),
        })
    }

    /// Create a new `Run` with multiple [`Arg`]
    ///
    /// Specifying multiple [`Arg`] arguments is actually just a short-hand for specifying multiple
    /// [`Run`]s with the same [`Options`] and environment variables.
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
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(Run::with_args([
    ///             Arg::empty("empty foo"),
    ///             Arg::new("foo", &["foo"])
    ///         ]));
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn with_args<I: AsRef<Arg>, T: AsRef<[I]>>(args: T) -> Self {
        let args = args.as_ref();
        Self(internal::RunnerRun {
            cmd: None,
            args: args.iter().map(|a| a.as_ref().into()).collect(),
            opts: None,
            envs: Vec::default(),
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
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo"))
    ///                 .arg(Arg::new("foo", &["foo"]))
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn arg<T: Into<internal::RunnerArg>>(&mut self, arg: T) -> &mut Self {
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
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
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
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    pub fn args<I: AsRef<Arg>, T: AsRef<[I]>>(&mut self, args: T) -> &mut Self {
        self.0
            .args
            .extend(args.as_ref().iter().map(|a| a.as_ref().into()));
        self
    }

    /// Add an environment variable available in the `cmd` of this `Run`
    ///
    /// An environment variable can be a `KEY=VALUE` pair or `KEY`. In the latter case this variable
    /// is a pass-through environment variable. Usually, the environment of the `cmd` is cleared but
    /// specifying pass-through variables makes this environment variable available to the `cmd` as
    /// it actually appeared in the root environment. Pass-through environment variables are ignored
    /// if they don't exist in the root environment.
    ///
    /// # Examples
    ///
    /// This'll define an environment variable `MY_ENV=42` which will be available in the `my-exe`
    /// binary
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo")).env("MY_ENV=42")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    ///
    /// If the `HOME=/home/my` variable is present in the original environment, the following will
    /// pass through the `HOME` variable to the `my-exe` binary with the original value `/home/my`
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo")).env("HOME")
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    pub fn env<T: Into<String>>(&mut self, env: T) -> &mut Self {
        self.0.envs.push(env.into());
        self
    }

    /// Add multiple environment variables available in the `cmd` of this `Run`
    ///
    /// See also [`Run::env`] for more details.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::empty("empty foo")).envs(["HOME", "MY_ENV=42"])
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    pub fn envs<I: AsRef<str>, T: AsRef<[I]>>(&mut self, envs: T) -> &mut Self {
        self.0
            .envs
            .extend(envs.as_ref().iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Change the default [`Options`] of this `Run`
    ///
    /// See also [`Options`] for more details and all possible options.
    ///
    /// # Examples
    ///
    /// The following would make the benchmark run of `my-exe` succeed if the benchmarked binary
    /// `my-exe` fails with an error when running it with the argument `foo`.
    ///
    /// ```rust
    /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, Options, ExitWith};
    ///
    /// binary_benchmark_group!(
    ///     name = my_exe_group;
    ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    ///         group.bench(
    ///             Run::with_arg(Arg::new("foo", &["foo"]))
    ///                 .options(
    ///                     Options::default().exit_with(ExitWith::Failure)
    ///                 )
    ///         );
    ///     }
    /// );
    /// # fn main() {
    /// # my_exe_group::my_exe_group(&mut BinaryBenchmarkGroup::default());
    /// # }
    /// ```
    pub fn options<T: Into<internal::RunnerOptions>>(&mut self, options: T) -> &mut Self {
        self.0.opts = Some(options.into());
        self
    }
}

/// The arguments needed for [`Run`] which are passed to the benchmarked binary
#[derive(Debug, Clone)]
pub struct Arg(internal::RunnerArg);

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
    pub fn new<T: Into<String>, I: AsRef<OsStr>, U: AsRef<[I]>>(id: T, args: U) -> Self {
        Self(internal::RunnerArg {
            id: Some(id.into()),
            args: args
                .as_ref()
                .iter()
                .map(|s| s.as_ref().to_owned())
                .collect(),
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
    pub fn empty<T: Into<String>>(id: T) -> Self {
        Self(internal::RunnerArg {
            id: Some(id.into()),
            args: vec![],
        })
    }
}

/// A builder for `Options`, applied to each benchmark [`Run`] of a benchmarked binary
#[derive(Debug, Default, Clone)]
pub struct Options(internal::RunnerOptions);

impl Options {
    /// If false, don't clear the environment variables before running the benchmark (Default: true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    ///
    /// let options = Options::default().env_clear(false);
    /// ```
    pub fn env_clear(&mut self, value: bool) -> &mut Self {
        self.0.env_clear = value;
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
    /// use std::path::PathBuf;
    ///
    /// use iai_callgrind::Options;
    ///
    /// let options: &mut Options = Options::default().current_dir(PathBuf::from("/tmp"));
    /// ```
    ///
    /// and the following will change the current directory to `fixtures` assuming it is
    /// contained in the root of the sandbox
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    ///
    /// let options: &mut Options = Options::default().current_dir("fixtures");
    /// ```
    pub fn current_dir<T: Into<PathBuf>>(&mut self, value: T) -> &mut Self {
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
    /// use iai_callgrind::Options;
    ///
    /// let options: &mut Options = Options::default().entry_point("my_exe::main");
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
    pub fn entry_point<T: Into<String>>(&mut self, value: T) -> &mut Self {
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
    /// use iai_callgrind::{ExitWith, Options};
    ///
    /// let options: &mut Options = Options::default().exit_with(ExitWith::Code(100));
    /// ```
    ///
    /// If a benchmark is expected to fail, but the exit code doesn't matter:
    ///
    /// ```rust
    /// use iai_callgrind::{ExitWith, Options};
    ///
    /// let options: &mut Options = Options::default().exit_with(ExitWith::Failure);
    /// ```
    pub fn exit_with<T: Into<internal::RunnerExitWith>>(&mut self, value: T) -> &mut Self {
        self.0.exit_with = Some(value.into());
        self
    }
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
pub struct Fixtures(internal::RunnerFixtures);

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
    pub fn new<T: Into<PathBuf>>(path: T) -> Self {
        Self(internal::RunnerFixtures {
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

/// An id for an [`Arg`] which can be used to produce unique ids from parameters
#[derive(Debug, Clone)]
pub struct BenchmarkId {
    id: String,
}

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
    pub fn new<T: AsRef<str>, P: Display>(id: T, parameter: P) -> Self {
        Self {
            id: format!("{}_{parameter}", id.as_ref()),
        }
    }
}

impl From<BenchmarkId> for String {
    fn from(value: BenchmarkId) -> Self {
        value.id
    }
}

/// Setting of [`Options::exit_with`] to set the expected exit status of a benchmarked binary
///
/// Per default, the benchmarked binary is expected to succeed, but if a benchmark is expected to
/// fail, setting this option is required.
///
/// # Examples
///
/// ```rust
/// use iai_callgrind::{Options, ExitWith};
///
/// iai_callgrind::main!(
///    run = cmd = "/bin/stat",
///        opts = Options::default().exit_with(ExitWith::Code(1)),
///        id = "file not exist", args = ["file does not exist"];
/// );
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

impl From<ExitWith> for internal::RunnerExitWith {
    fn from(value: ExitWith) -> Self {
        match value {
            ExitWith::Success => Self::Success,
            ExitWith::Failure => Self::Failure,
            ExitWith::Code(c) => Self::Code(c),
        }
    }
}

impl From<&ExitWith> for internal::RunnerExitWith {
    fn from(value: &ExitWith) -> Self {
        match value {
            ExitWith::Success => Self::Success,
            ExitWith::Failure => Self::Failure,
            ExitWith::Code(c) => Self::Code(*c),
        }
    }
}

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

impl_traits!(BinaryBenchmarkGroup, internal::RunnerBinaryBenchmarkGroup);
impl_traits!(BinaryBenchmarkConfig, internal::RunnerConfig);
impl_traits!(
    LibraryBenchmarkConfig,
    internal::RunnerLibraryBenchmarkConfig
);
impl_traits!(Options, internal::RunnerOptions);
impl_traits!(Run, internal::RunnerRun);
impl_traits!(Arg, internal::RunnerArg);
impl_traits!(Fixtures, internal::RunnerFixtures);
