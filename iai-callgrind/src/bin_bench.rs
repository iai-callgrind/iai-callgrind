use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;

use crate::internal;
///
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

/// `Run` let's you set up and configure a benchmark run of a binary
#[derive(Debug, Default, Clone)]
pub struct Run(internal::InternalRun);

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

impl From<BenchmarkId> for String {
    fn from(value: BenchmarkId) -> Self {
        value.id
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
    /// See also [`crate::LibraryBenchmarkConfig::pass_through_env`].
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
    /// # Glob Patterns
    ///
    /// Glob patterns are allowed in the same way as callgrind's --toggle-collect option allows glob
    /// patterns. Note the pattern matches from start to end of the path. For example `*::main`
    /// matches
    ///
    /// * `my_exe::main`
    /// * `other::main`
    ///
    /// but not:
    ///
    /// * `main`
    /// * `other::main::sub`
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

    /// Option to produce flamegraphs from callgrind output using the [`crate::FlamegraphConfig`]
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
        self.0.flamegraph_config = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`crate::RegressionConfig`]
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
        self.0.regression_config = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`crate::Tool`] in addition to callgrind
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

    /// Add multiple configurations to run valgrind [`crate::Tool`]s in addition to callgrind
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
    ///
    /// See also [`crate::LibraryBenchmarkConfig::tool_override`] for more details.
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
    ///
    /// See also [`crate::LibraryBenchmarkConfig::tool_override`] for more details.
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

    /// Option to produce flamegraphs from callgrind output using the [`crate::FlamegraphConfig`]
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
        self.0.config.flamegraph_config = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`crate::RegressionConfig`]
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
        self.0.config.regression_config = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`crate::Tool`] in addition to callgrind
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

    /// Add multiple configurations to run valgrind [`crate::Tool`]s in addition to callgrind
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
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
