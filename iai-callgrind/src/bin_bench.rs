use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::path::PathBuf;

use derive_more::AsRef;
use iai_callgrind_macros::IntoInner;

use crate::{internal, Stdin, Stdio};
// TODO: UPDATE DOCS

/// An id for an [`Arg`] which can be used to produce unique ids from parameters
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BenchmarkId(String);

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
#[derive(Debug, Default, Clone, IntoInner, AsRef)]
pub struct BinaryBenchmarkConfig(internal::InternalBinaryBenchmarkConfig);

/// TODO: UPDATE DOCUMENTATION
#[derive(Debug, Default, Clone)]
pub struct BinaryBenchmarkGroup {
    /// TODO: DOCUMENTATION
    pub binary_benchmarks: Vec<BinaryBenchmark>,
}

/// TODO: DOCUMENTATION
#[derive(Debug, Clone)]
pub struct Bench {
    /// TODO: DOCUMENTATION
    pub id: BenchmarkId,
    /// TODO: DOCUMENTATION
    pub commands: Vec<Command>,
    /// TODO: DOCUMENTATION
    pub config: Option<BinaryBenchmarkConfig>,
    /// TODO: DOCUMENTATION
    pub setup: Option<fn()>,
    /// TODO: DOCUMENTATION
    pub teardown: Option<fn()>,
}

/// TODO: DOCUMENTATION
#[derive(Debug, Clone)]
pub struct BinaryBenchmark {
    /// TODO: DOCUMENTATION
    pub id: BenchmarkId,
    /// TODO: DOCUMENTATION
    pub config: Option<BinaryBenchmarkConfig>,
    /// TODO: DOCUMENTATION
    pub benches: Vec<Bench>,
    /// TODO: DOCUMENTATION
    pub setup: Option<fn()>,
    /// TODO: DOCUMENTATION
    pub teardown: Option<fn()>,
}

/// TODO: DOCUMENTATION
#[derive(Debug, Default, Clone, IntoInner, AsRef)]
pub struct Command(internal::InternalCommand);

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
#[derive(Debug, Clone, Copy)]
pub enum ExitWith {
    /// Exit with success is similar to `ExitCode(0)`
    Success,
    /// Exit with failure is similar to setting the `ExitCode` to something different than `0`
    Failure,
    /// The exact `ExitCode` of the benchmark run
    Code(i32),
}

/// TODO: DOCUMENTATION
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Sandbox(internal::InternalSandbox);

impl Bench {
    /// TODO: DOCUMENTATION
    pub fn new<T>(id: T) -> Self
    where
        T: Into<BenchmarkId>,
    {
        Self {
            id: id.into(),
            config: None,
            commands: vec![],
            setup: None,
            teardown: None,
        }
    }

    /// TODO: DOCUMENTATION
    pub fn config<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<BinaryBenchmarkConfig>,
    {
        self.config = Some(config.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn command<T>(&mut self, command: T) -> &mut Self
    where
        T: Into<Command>,
    {
        self.commands.push(command.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn commands<I, T>(&mut self, commands: T) -> &mut Self
    where
        I: Into<Command>,
        T: IntoIterator<Item = I>,
    {
        self.commands.extend(commands.into_iter().map(Into::into));
        self
    }

    /// TODO: DOCUMENTATION
    pub fn setup(&mut self, setup: fn()) -> &mut Self {
        self.setup = Some(setup);
        self
    }

    /// TODO: DOCUMENTATION
    pub fn teardown(&mut self, teardown: fn()) -> &mut Self {
        self.teardown = Some(teardown);
        self
    }
}

impl From<&mut Bench> for Bench {
    fn from(value: &mut Bench) -> Self {
        value.clone()
    }
}

impl From<&Bench> for Bench {
    fn from(value: &Bench) -> Self {
        value.clone()
    }
}

impl BenchmarkId {
    /// TODO: UPDATE DOCUMENTATION
    /// Create a new `BenchmarkId` with a parameter
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
    pub fn with_parameter<T, P>(id: T, parameter: P) -> Self
    where
        T: AsRef<str>,
        P: Display,
    {
        Self(format!("{}_{parameter}", id.as_ref()))
    }

    /// TODO: DOCUMENTATION
    pub fn new<T>(id: T) -> Self
    where
        T: Into<String>,
    {
        Self(id.into())
    }

    /// TODO: DOCUMENTATION
    /// Returns the validate of this [`BenchmarkId`].
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn validate(&self) -> Result<(), String> {
        if self.0.is_empty() {
            return Err("Invalid id: Cannot be empty".to_owned());
        }

        let mut bytes = self.0.bytes();
        // This unwrap is safe, since we just checked that the string is not empty
        let first = bytes.next().unwrap();

        if first.is_ascii_alphabetic() || first == b'_' {
            for byte in bytes {
                if byte.is_ascii() {
                    if !(byte.is_ascii_alphanumeric() || byte == b'_') {
                        return Err(format!(
                            "Invalid id '{}': Invalid character '{}'",
                            &self.0,
                            char::from(byte)
                        ));
                    }
                } else {
                    return Err(format!(
                        "Invalid id '{}': Encountered non-ascii character",
                        &self.0
                    ));
                }
            }
        } else if first.is_ascii() {
            return Err(format!(
                "Invalid id '{}': As first character is '{}' not allowed",
                &self.0,
                char::from(first)
            ));
        } else {
            return Err(format!(
                "Invalid id '{}': Encountered non-ascii character as first character",
                &self.0
            ));
        }
        Ok(())
    }
}

impl Display for BenchmarkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// TODO: REMOVE in favour of TryFrom
impl From<BenchmarkId> for String {
    fn from(value: BenchmarkId) -> Self {
        value.0
    }
}

// TODO: CHANGE THIS to TryFrom
impl<T> From<T> for BenchmarkId
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        Self(value.as_ref().to_owned())
    }
}

impl BinaryBenchmark {
    /// TODO: DOCUMENTATION
    pub fn new<T>(id: T) -> Self
    where
        T: Into<BenchmarkId>,
    {
        Self {
            id: id.into(),
            config: None,
            benches: vec![],
            setup: None,
            teardown: None,
        }
    }

    /// TODO: DOCUMENTATION
    pub fn config<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<BinaryBenchmarkConfig>,
    {
        self.config = Some(config.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn bench<T>(&mut self, bench: T) -> &mut Self
    where
        T: Into<Bench>,
    {
        self.benches.push(bench.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn setup(&mut self, setup: fn()) -> &mut Self {
        self.setup = Some(setup);
        self
    }

    /// TODO: DOCUMENTATION
    pub fn teardown(&mut self, teardown: fn()) -> &mut Self {
        self.teardown = Some(teardown);
        self
    }
}

impl From<&mut BinaryBenchmark> for BinaryBenchmark {
    fn from(value: &mut BinaryBenchmark) -> Self {
        value.clone()
    }
}

impl From<&BinaryBenchmark> for BinaryBenchmark {
    fn from(value: &BinaryBenchmark) -> Self {
        value.clone()
    }
}

impl BinaryBenchmarkConfig {
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

    // TODO: Update Documentation
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
    pub fn sandbox<T>(&mut self, sandbox: T) -> &mut Self
    where
        T: Into<internal::InternalSandbox>,
    {
        self.0.sandbox = Some(sandbox.into());
        self
    }

    /// Adjust, enable or disable the truncation of the description in the iai-callgrind output
    ///
    /// The default is to truncate the description to the size of 50 ascii characters. A `None`
    /// value disables the truncation entirely and a `Some` value will truncate the description to
    /// the given amount of characters excluding the ellipsis.
    ///
    /// To clearify which part of the output is meant by `DESCRIPTION`:
    ///
    /// ```text
    /// benchmark_file::group_name id:DESCRIPTION
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
    /// use iai_callgrind::{main, BinaryBenchmarkConfig};
    /// # use iai_callgrind::binary_benchmark_group;
    /// # binary_benchmark_group!(
    /// #    name = some_group;
    /// #    benchmark = |"", group: &mut BinaryBenchmarkGroup| {}
    /// # );
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().truncate_description(None);
    ///     binary_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn truncate_description(&mut self, value: Option<usize>) -> &mut Self {
        self.0.truncate_description = Some(value);
        self
    }
}

impl BinaryBenchmarkGroup {
    /// TODO: DOCUMENTATION
    pub fn binary_benchmark<T>(&mut self, binary_benchmark: T) -> &mut Self
    where
        T: Into<BinaryBenchmark>,
    {
        self.binary_benchmarks.push(binary_benchmark.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn binary_benchmarks<I, T>(&mut self, binary_benchmarks: T) -> &mut Self
    where
        I: Into<BinaryBenchmark>,
        T: IntoIterator<Item = I>,
    {
        self.binary_benchmarks
            .extend(binary_benchmarks.into_iter().map(Into::into));
        self
    }
}

// TODO: WAIT FUNCTION which tells iai-callgrind to wait for this process only this specific
// amount of seconds instead of blocking forever. Also, add this method to
// `BinaryBenchmarkConfig` (and `LibraryBenchmarkConfig`??)
//
// TODO: DELAY FUNCTION to delay the start of the main process if required.
impl Command {
    /// TODO: DOCUMENTATION
    pub fn new<T>(path: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        Self(internal::InternalCommand {
            path: PathBuf::from(path.as_ref()),
            ..Default::default()
        })
    }

    /// TODO: DOCUMENTATION
    pub fn arg<T>(&mut self, arg: T) -> &mut Self
    where
        T: Into<OsString>,
    {
        self.0.args.push(arg.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: Into<OsString>,
        T: IntoIterator<Item = I>,
    {
        self.0.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// TODO: DOCUMENTATION
    pub fn stdin<T>(&mut self, stdio: T) -> &mut Self
    where
        T: Into<Stdin>,
    {
        self.0.stdin = Some(stdio.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn stdout<T>(&mut self, stdio: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.0.stdout = Some(stdio.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn stderr<T>(&mut self, stdio: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.0.stderr = Some(stdio.into());
        self
    }

    /// TODO: DOCUMENTATION
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

    /// TODO: DOCUMENTATION
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.0
            .config
            .envs
            .extend(vars.into_iter().map(|(k, v)| (k.into(), Some(v.into()))));
        self
    }

    // TODO: YES OR NO??
    // /// Specify a pass-through environment variable
    // ///
    // /// See also [`BinaryBenchmarkConfig::pass_through_env`]
    // ///
    // /// # Examples
    // ///
    // /// Here, we chose to pass-through the original value of the `HOME` variable:
    // ///
    // /// ```rust
    // /// # use iai_callgrind::main;
    // /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    // ///
    // /// binary_benchmark_group!(
    // ///     name = my_group;
    // ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    // ///         group.bench(
    // ///             Run::with_arg(Arg::empty("empty foo"))
    // ///                 .pass_through_env("HOME")
    // ///         );
    // ///     }
    // /// );
    // /// # fn main() {
    // /// # main!(binary_benchmark_groups = my_group);
    // /// # }
    // /// ```
    // pub fn pass_through_env<K>(&mut self, key: K) -> &mut Self
    // where
    //     K: Into<OsString>,
    // {
    //     self.0.config.envs.push((key.into(), None));
    //     self
    // }
    //
    // /// Specify multiple pass-through environment variables
    // ///
    // /// See also [`BinaryBenchmarkConfig::pass_through_env`].
    // ///
    // /// # Examples
    // ///
    // /// ```rust
    // /// # use iai_callgrind::main;
    // /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    // ///
    // /// binary_benchmark_group!(
    // ///     name = my_group;
    // ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    // ///         group.bench(
    // ///             Run::with_arg(Arg::empty("empty foo"))
    // ///                 .pass_through_envs(["HOME", "USER"])
    // ///         );
    // ///     }
    // /// );
    // /// # fn main() {
    // /// # main!(binary_benchmark_groups = my_group);
    // /// # }
    // /// ```
    // pub fn pass_through_envs<K, T>(&mut self, envs: T) -> &mut Self
    // where
    //     K: Into<OsString>,
    //     T: IntoIterator<Item = K>,
    // {
    //     self.0
    //         .config
    //         .envs
    //         .extend(envs.into_iter().map(|k| (k.into(), None)));
    //     self
    // }
    // /// If false, don't clear the environment variables before running the benchmark (Default:
    // true) ///
    // /// # Examples
    // ///
    // /// ```rust
    // /// # use iai_callgrind::main;
    // /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run};
    // ///
    // /// binary_benchmark_group!(
    // ///     name = my_group;
    // ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    // ///         group.bench(
    // ///             Run::with_arg(Arg::empty("empty foo"))
    // ///                 .env_clear(false)
    // ///         );
    // ///     }
    // /// );
    // /// # fn main() {
    // /// # main!(binary_benchmark_groups = my_group);
    // /// # }
    // /// ```
    // pub fn env_clear(&mut self, value: bool) -> &mut Self {
    //     self.0.config.env_clear = Some(value);
    //     self
    // }

    /// TODO: UPDATE DOCUMENTATION
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

    /// TODO: UPDATE DOCUMENTATION
    // /// Set the expected exit status [`ExitWith`] of a benchmarked binary
    // ///
    // /// See also [`BinaryBenchmarkConfig::exit_with`]
    // ///
    // /// # Examples
    // ///
    // /// If the benchmark is expected to fail with a specific exit code, for example `100`:
    // ///
    // /// ```rust
    // /// # use iai_callgrind::main;
    // /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, ExitWith};
    // ///
    // /// binary_benchmark_group!(
    // ///     name = my_group;
    // ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    // ///         group.bench(
    // ///             Run::with_arg(Arg::empty("empty foo"))
    // ///                 .exit_with(ExitWith::Code(100))
    // ///         );
    // ///     }
    // /// );
    // /// # fn main() {
    // /// # main!(binary_benchmark_groups = my_group);
    // /// # }
    // /// ```
    // ///
    // /// If a benchmark is expected to fail, but the exit code doesn't matter:
    // ///
    // /// ```rust
    // /// # use iai_callgrind::main;
    // /// use iai_callgrind::{binary_benchmark_group, Arg, BinaryBenchmarkGroup, Run, ExitWith};
    // ///
    // /// binary_benchmark_group!(
    // ///     name = my_group;
    // ///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
    // ///         group.bench(
    // ///             Run::with_arg(Arg::empty("empty foo"))
    // ///                 .exit_with(ExitWith::Failure)
    // ///         );
    // ///     }
    // /// );
    // /// # fn main() {
    // /// # main!(binary_benchmark_groups = my_group);
    // /// # }
    // /// ```
    pub fn exit_with(&mut self, exit_with: ExitWith) -> &mut Self {
        self.0.config.exit_with = Some(exit_with.into());
        self
    }

    /// TODO: DOCUMENTATION
    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl From<&mut Command> for Command {
    fn from(value: &mut Command) -> Self {
        value.clone()
    }
}

impl From<&Command> for Command {
    fn from(value: &Command) -> Self {
        value.clone()
    }
}

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

/// TODO: ADD `follow_symlinks` and maybe others. See `InternalSandbox`
impl Sandbox {
    /// TODO: DOCUMENTATION
    pub fn new(enabled: bool) -> Self {
        Self(internal::InternalSandbox {
            enabled: Some(enabled),
            ..Default::default()
        })
    }

    /// TODO: DOCUMENTATION
    pub fn fixtures<I, T>(&mut self, paths: T) -> &mut Self
    where
        I: Into<PathBuf>,
        T: IntoIterator<Item = I>,
    {
        self.0.fixtures.extend(paths.into_iter().map(Into::into));
        self
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::simple_invalid("-")]
    #[case::when_0_char_minus_1("a\x2f")]
    #[case::when_9_char_plus_1("a\x3a")]
    #[case::when_big_a_char_minus_1("\x40")]
    #[case::when_big_z_char_plus_1("\x5b")]
    #[case::when_low_a_char_minus_1("\x60")]
    #[case::when_low_z_char_plus_1("\x7b")]
    #[case::invalid_2nd("a-")]
    #[case::invalid_3rd("ab-")]
    #[case::all_invalid("---")]
    #[case::number_as_first("0")]
    // This would be a valid rust identifier, but we don't allow the whole set
    #[case::non_ascii_1st("µ")]
    #[case::non_ascii_2nd("aµ")]
    #[case::non_ascii_3rd("aaµ")]
    #[case::non_ascii_middle("aµa")]
    fn test_benchmark_id_validate_when_error(#[case] id: &str) {
        let id = BenchmarkId::new(id);
        assert!(id.validate().is_err());
    }

    #[rstest]
    #[case::lowercase_a("a")]
    #[case::lowercase_b("b")]
    #[case::lowercase_m("m")]
    #[case::lowercase_y("y")]
    #[case::lowercase_z("z")]
    #[case::uppercase_a("A")]
    #[case::uppercase_b("B")]
    #[case::uppercase_n("N")]
    #[case::uppercase_y("Y")]
    #[case::uppercase_z("Z")]
    #[case::zero_2nd("a0")]
    #[case::one_2nd("a1")]
    #[case::eight_2nd("a8")]
    #[case::nine_2nd("a9")]
    #[case::number_middle("b4t")]
    #[case::underscore("_")]
    #[case::only_underscore("___")]
    #[case::underscore_last("a_")]
    #[case::mixed_all("auAEwer9__2xcd")]
    fn test_benchmark_id_validate(#[case] id: &str) {
        let id = BenchmarkId::new(id);
        assert!(id.validate().is_ok());
    }

    #[rstest]
    #[case::empty("", "Invalid id: Cannot be empty")]
    #[case::non_ascii_first(
        "µ",
        "Invalid id 'µ': Encountered non-ascii character as first character"
    )]
    #[case::multibyte_middle("aµ", "Invalid id 'aµ': Encountered non-ascii character")]
    #[case::non_ascii_middle("a-", "Invalid id 'a-': Invalid character '-'")]
    #[case::invalid_first("-", "Invalid id '-': As first character is '-' not allowed")]
    fn test_benchmark_id_validate_error_message(#[case] id: &str, #[case] message: &str) {
        let id = BenchmarkId::new(id);
        assert_eq!(
            id.validate()
                .expect_err("Validation should return an error"),
            message
        );
    }
}
