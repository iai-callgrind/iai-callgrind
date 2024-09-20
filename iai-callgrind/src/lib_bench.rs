use std::ffi::OsString;

use derive_more::AsRef;
use iai_callgrind_macros::IntoInner;

use crate::{internal, EntryPoint};

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
#[derive(Debug, Default, IntoInner, AsRef)]
pub struct LibraryBenchmarkConfig(internal::InternalLibraryBenchmarkConfig);

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
            raw_callgrind_args: internal::InternalRawArgs::from_iter(args),
            envs: Vec::default(),
            flamegraph_config: Option::default(),
            regression_config: Option::default(),
            tools: internal::InternalTools::default(),
            tools_override: Option::default(),
            output_format: Option::default(),
            entry_point: Option::default(),
        })
    }

    /// Add callgrind arguments to this `LibraryBenchmarkConfig`
    ///
    /// The arguments don't need to start with a flag: `--toggle-collect=some` or
    /// `toggle-collect=some` are both understood.
    ///
    /// Not all callgrind arguments are understood by `iai-callgrind` or cause problems in
    /// `iai-callgrind` if they would be applied. `iai-callgrind` will issue a warning in such
    /// cases. Most of the defaults can be overwritten. The default settings are:
    ///
    /// * `--I1=32768,8,64`
    /// * `--D1=32768,8,64`
    /// * `--LL=8388608,16,64`
    /// * `--cache-sim=yes`
    /// * `--toggle-collect=...` (see also [`LibraryBenchmarkConfig::entry_point`])
    /// * `--collect-atstart=no`
    /// * `--compress-pos=no`
    /// * `--compress-strings=no`
    ///
    /// Note that `toggle-collect` is an array and the default [`EntryPoint`] for library benchmarks
    /// is the benchmark function.
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

    /// Option to produce flamegraphs from callgrind output using the [`crate::FlamegraphConfig`]
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
        self.0.flamegraph_config = Some(config.into());
        self
    }

    /// Enable performance regression checks with a [`crate::RegressionConfig`]
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
        self.0.regression_config = Some(config.into());
        self
    }

    /// Add a configuration to run a valgrind [`crate::Tool`] in addition to callgrind
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

    /// Add multiple configurations to run valgrind [`crate::Tool`]s in addition to callgrind
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
    ///
    /// Usually, if specifying [`crate::Tool`] configurations with [`LibraryBenchmarkConfig::tool`]
    /// these tools are appended to the configuration of a [`LibraryBenchmarkConfig`] of
    /// higher-levels. Specifying a [`crate::Tool`] with this method overrides previously defined
    /// configurations.
    ///
    /// Note that [`crate::Tool`]s specified with [`LibraryBenchmarkConfig::tool`] will be ignored,
    /// if in the very same `LibraryBenchmarkConfig`, [`crate::Tool`]s are specified by this method
    /// (or [`LibraryBenchmarkConfig::tools_override`]).
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

    /// Override previously defined configurations of valgrind [`crate::Tool`]s
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

    /// Set or unset the entry point for a benchmark
    ///
    /// Iai-Callgrind sets the [`--toggle-collect`] argument of callgrind to the benchmark function
    /// which we call [`EntryPoint::Default`]. Specifying a `--toggle-collect` argument, sets
    /// automatically `--collect-at-start=no`. This ensures that only the metrics from the benchmark
    /// itself are collected and not the `setup` or `teardown` or anything before/after the
    /// benchmark function.
    ///
    ///
    /// However, there are cases when the default toggle is not enough [`EntryPoint::Custom`] or in
    /// the way [`EntryPoint::None`].
    ///
    /// Setting [`EntryPoint::Custom`] is convenience for disabling the entry point with
    /// [`EntryPoint::None`] and setting `--toggle-collect=CUSTOM_ENTRY_POINT` in
    /// [`LibraryBenchmarkConfig::raw_callgrind_args`]. [`EntryPoint::Custom`] can be useful if you
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
    /// In the example above [`EntryPoint::Default`] is active, so the counting of events starts
    /// when the `some_bench` function is entered. In `to_be_benchmarked`, the client request
    /// `start_instrumentation` does effectively nothing and `stop_instrumentation` will stop the
    /// event counting as requested. This is most likely not what you intended. The event counting
    /// should start with `start_instrumentation`. To achieve this, you can set [`EntryPoint::None`]
    /// which removes the default toggle, but also `--collect-at-start=no`. So, you need to specify
    /// `--collect-at-start=no` in [`LibraryBenchmarkConfig::raw_callgrind_args`]. The example would
    /// then look like this:
    /// ```rust
    /// use std::hint::black_box;
    ///
    /// use iai_callgrind::{library_benchmark, EntryPoint, LibraryBenchmarkConfig};
    /// # use iai_callgrind::{library_benchmark_group, main};
    /// # fn to_be_benchmarked() -> u64 { 10 }
    ///
    /// // ...
    ///
    /// #[library_benchmark(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .raw_callgrind_args(["--collect-at-start=no"])
    ///         .entry_point(EntryPoint::None)
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
    pub fn entry_point<T>(&mut self, entry_point: T) -> &mut Self
    where
        T: Into<EntryPoint>,
    {
        self.0.entry_point = Some(entry_point.into());
        self
    }

    /// TODO: DOCS
    pub fn output_format<T>(&mut self, output_format: T) -> &mut Self
    where
        T: Into<internal::InternalOutputFormat>,
    {
        self.0.output_format = Some(output_format.into());
        self
    }
}
