use std::ffi::OsString;

use derive_more::AsRef;
use iai_callgrind_macros::IntoInner;
use iai_callgrind_runner::api::ValgrindTool;

use crate::__internal;

/// The main configuration of a library benchmark.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group};
/// use iai_callgrind::{LibraryBenchmarkConfig, main, Callgrind};
/// # #[library_benchmark]
/// # fn some_func() {}
/// # library_benchmark_group!(name = some_group; benchmarks = some_func);
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .tool(Callgrind::with_args(["toggle-collect=something"]));
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
#[derive(Debug, Default, IntoInner, AsRef, Clone)]
pub struct LibraryBenchmarkConfig(__internal::InternalLibraryBenchmarkConfig);

// BinaryBenchmarkConfig
impl LibraryBenchmarkConfig {
    /// Pass valgrind arguments to all tools
    ///
    /// Only core [valgrind
    /// arguments](https://valgrind.org/docs/manual/manual-core.html#manual-core.options) are
    /// allowed.
    ///
    /// These arguments can be overwritten by tool specific arguments for example with
    /// [`crate::Callgrind::args`]
    ///
    /// # Examples
    ///
    /// Specify `--trace-children=no` for all configured tools (including callgrind):
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark_group, library_benchmark};
    /// # #[library_benchmark] fn bench_me() {}
    /// # library_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = bench_me
    /// # );
    /// use iai_callgrind::{main, LibraryBenchmarkConfig, Dhat};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .valgrind_args(["--trace-children=no"])
    ///         .tool(Dhat::default());
    ///     library_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    ///
    /// Overwrite the valgrind argument `--num-callers=25` for `DHAT` with `--num-callers=30`:
    ///
    /// ```rust
    /// # use iai_callgrind::{library_benchmark_group, library_benchmark};
    /// # #[library_benchmark] fn bench_me() {}
    /// # library_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = bench_me
    /// # );
    /// use iai_callgrind::{main, LibraryBenchmarkConfig, Dhat};
    ///
    /// # fn main() {
    /// main!(
    ///     config = LibraryBenchmarkConfig::default()
    ///         .valgrind_args(["--num-callers=25"])
    ///         .tool(Dhat::with_args(["--num-callers=30"]));
    ///     library_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn valgrind_args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.valgrind_args.extend_ignore_flag(args);
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
    ///         LibraryBenchmarkConfig::default()
    ///             .envs([("MY_CUSTOM_VAR", "SOME_VALUE"), ("FOO", "BAR")]);
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
    /// Here, we chose to pass through the original value of the `HOME` variable:
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

    /// Add a configuration to run a valgrind tool in addition to callgrind
    ///
    /// # Examples
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
    pub fn tool<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<__internal::InternalTool>,
    {
        self.0.tools.update(tool.into());
        self
    }

    /// Override previously defined configurations of valgrind tools
    ///
    /// Usually, if specifying tool configurations with [`LibraryBenchmarkConfig::tool`] these tools
    /// are appended to the configuration of a [`LibraryBenchmarkConfig`] of higher-levels.
    /// Specifying a tool with this method overrides previously defined configurations.
    ///
    /// # Examples
    ///
    /// The following will run `DHAT` and `Massif` (and the default callgrind) for all benchmarks in
    /// `main!` besides for `some_func` which will just run `Memcheck` (and callgrind).
    ///
    /// ```rust
    /// use iai_callgrind::{
    ///     main, library_benchmark, library_benchmark_group, LibraryBenchmarkConfig, Memcheck,
    ///     Massif, Dhat
    /// };
    ///
    /// #[library_benchmark(config = LibraryBenchmarkConfig::default()
    ///     .tool_override(Memcheck::default())
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
    ///         .tool(Dhat::default())
    ///         .tool(Massif::default());
    ///     library_benchmark_groups = some_group
    /// );
    /// # }
    /// ```
    pub fn tool_override<T>(&mut self, tool: T) -> &mut Self
    where
        T: Into<__internal::InternalTool>,
    {
        self.0
            .tools_override
            .get_or_insert(__internal::InternalTools::default())
            .update(tool.into());
        self
    }

    /// Configure the [`crate::OutputFormat`] of the terminal output of Iai-Callgrind
    ///
    /// # Examples
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
    pub fn output_format<T>(&mut self, output_format: T) -> &mut Self
    where
        T: Into<__internal::InternalOutputFormat>,
    {
        self.0.output_format = Some(output_format.into());
        self
    }

    /// TODO: DOCS
    pub fn default_tool(&mut self, tool: ValgrindTool) -> &mut Self {
        self.0.default_tool = Some(tool);
        self
    }
}
