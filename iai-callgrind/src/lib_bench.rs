use std::ffi::OsString;

use crate::internal;

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
