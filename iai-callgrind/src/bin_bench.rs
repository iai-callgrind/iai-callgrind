use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use derive_more::AsRef;
use iai_callgrind_macros::IntoInner;

use crate::{internal, Stdin, Stdio};

/// [low level api](`crate::binary_benchmark_group`) only: Create a new benchmark id
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BenchmarkId(String);

/// The configuration of a binary benchmark
///
/// The [`BinaryBenchmarkConfig`] can be specified at multiple levels and configures the benchmarks
/// at this level. For example a [`BinaryBenchmarkConfig`] at (`main`)[`crate::main`] level
/// configures all benchmarks. A configuration at [`group`](crate::binary_benchmark_group) level
/// configures all benchmarks in this group inheriting the configuration of the `main` level and if
/// not specified otherwise overwrites the values of the `main` configuration if the option is
/// specified in both [`BinaryBenchmarkConfig`]s. The more deeper levels are the
/// (`#[binary_benchmark] attribute`)[`crate::binary_benchmark`], the `#[bench]` and the
/// `#[benches]` attribute.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::binary_benchmark_group;
/// # binary_benchmark_group!(name = some_group; benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
/// use iai_callgrind::{BinaryBenchmarkConfig, main};
///
/// main!(
///     config = BinaryBenchmarkConfig::default().raw_callgrind_args(["toggle-collect=something"]);
///     binary_benchmark_groups = some_group
/// );
/// ```
#[derive(Debug, Default, Clone, IntoInner, AsRef)]
pub struct BinaryBenchmarkConfig(internal::InternalBinaryBenchmarkConfig);

/// [low level api](`crate::binary_benchmark_group`) only: The top level struct to add binary
/// benchmarks to
///
/// This struct doesn't need to be instantiated by yourself. It is passed as mutable reference to
/// the expression in `benchmarks`.
///
/// ```rust
/// use iai_callgrind::binary_benchmark_group;
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = |_group: &mut BinaryBenchmarkGroup| {
///         // Access the BinaryBenchmarkGroup with the identifier `group` to add benchmarks to the
///         // group.
///         //
///         // group.binary_benchmark(/* BinaryBenchmark::new(...) */);
///     }
/// );
/// ```
#[derive(Debug, Default, PartialEq, Clone)]
pub struct BinaryBenchmarkGroup {
    /// All [`BinaryBenchmark`]s
    pub binary_benchmarks: Vec<BinaryBenchmark>,
}

/// [low level api](`crate::binary_benchmark_group`) only: This struct mirrors the `#[bench]` and
/// `#[benches]` attribute of a [`crate::binary_benchmark`]
#[derive(Debug, Clone, PartialEq)]
pub struct Bench {
    /// The [`BenchmarkId`] used to uniquely identify this benchmark within a [`BinaryBenchmark`]
    pub id: BenchmarkId,
    /// All [`Command`]s
    pub commands: Vec<Command>,
    /// An optional [`BinaryBenchmarkConfig`]
    ///
    /// This field stores the internal representation of the [`BinaryBenchmarkConfig`]. Use
    /// `BinaryBenchmarkConfig::into` to generate the internal configuration from a
    /// [`BinaryBenchmarkConfig`]
    pub config: Option<internal::InternalBinaryBenchmarkConfig>,
    /// The `setup` function to be executed before the [`Command`] is executed
    pub setup: Option<fn()>,
    /// The `teardown` function to be executed after the [`Command`] is executed
    pub teardown: Option<fn()>,
}

/// [low level api](`crate::binary_benchmark_group`) only: Mirror the [`crate::binary_benchmark`]
/// attribute
///
/// A `BinaryBenchmark` can be created in two ways. Either with [`BinaryBenchmark::new`]. Or via the
/// [`crate::binary_benchmark_attribute`] macro used with a function annotated with the
/// [`crate::binary_benchmark`] attribute. So, you can start with the high-level api using the
/// attribute and then go on in the low-level api.
///
/// # Examples
///
/// For examples using [`BinaryBenchmark::new`], see there. Here's an example using the
/// [`crate::binary_benchmark_attribute`]
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{
///     binary_benchmark, binary_benchmark_group, binary_benchmark_attribute, Bench
/// };
///
/// #[binary_benchmark]
/// #[bench::foo("foo")]
/// fn bench_binary(arg: &str) -> iai_callgrind::Command {
///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///         .arg(arg)
///         .build()
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
///         let mut binary_benchmark = binary_benchmark_attribute!(bench_binary);
///
///         // Continue and add another `Bench` to the `BinaryBenchmark`
///         binary_benchmark.bench(Bench::new("bar")
///             .command(iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///                 .arg("bar")
///             )
///         );
///
///         // Finally, add the `BinaryBenchmark` to the group
///         group
///             .binary_benchmark(binary_benchmark);
///     }
/// );
/// # fn main() {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryBenchmark {
    /// An id which has to be unique within the same [`BinaryBenchmarkGroup`]
    ///
    /// In the high-level api this is the name of the function which is annotated by
    /// [`crate::binary_benchmark`]
    pub id: BenchmarkId,
    /// An optional [`BinaryBenchmarkConfig`] which is applied to all [`Command`]s within this
    /// [`BinaryBenchmark`]
    pub config: Option<internal::InternalBinaryBenchmarkConfig>,
    /// All [`Bench`]es which were added to this [`BinaryBenchmark`]
    pub benches: Vec<Bench>,
    /// The default `setup` function for all [`Bench`]es within this [`BinaryBenchmark`]. It can be
    /// overwritten in a [`Bench`]
    pub setup: Option<fn()>,
    /// The default `teardown` function for all [`Bench`]es within this [`BinaryBenchmark`]. It can
    /// be overwritten in a [`Bench`]
    pub teardown: Option<fn()>,
}

/// A builder for a [`std::process::Command`] providing fine-grained control over how the
/// [`std::process::Command`] for the valgrind benchmark should be created.
///
/// The default configuration is generated with [`Command::new`] providing a path to an executable.
/// Adding a crate's binary is usually done with `env!("CARGO_BIN_EXE_<name>")` where `<name>` is
/// the name of the binary.
///
/// Suppose your crate's binary is called `my-echo`:
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::Command;
/// let command = Command::new(env!("CARGO_BIN_EXE_my-echo"));
/// ```
///
/// However, an iai-callgrind benchmark is not limited to a crate's binaries, it can be any
/// executable in the `$PATH`, or a absolute path to a binary.
#[derive(Debug, Default, Clone, PartialEq, IntoInner, AsRef)]
pub struct Command(internal::InternalCommand);

/// Set the expected exit status of a binary benchmark
///
/// Per default, the benchmarked binary is expected to succeed, but if a benchmark is expected to
/// fail, setting this option is required.
///
/// # Examples
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group};
/// # binary_benchmark_group!(
/// #    name = my_group;
/// #    benchmarks = |group: &mut BinaryBenchmarkGroup| {});
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
    /// without having to rely on a specific exit code
    Failure,
    /// The exact `ExitCode` of the benchmark run
    Code(i32),
}

/// Configure the sandboxing behaviour of binary benchmarks
#[derive(Debug, Clone, IntoInner, AsRef)]
pub struct Sandbox(internal::InternalSandbox);

impl Bench {
    /// Create a new `Bench` with an unique [`BenchmarkId`]
    ///
    /// If the provided [`BenchmarkId`] is invalid, `iai-callgrind` exits with an error.
    ///
    /// # Scope of uniqueness of the [`BenchmarkId`]
    ///
    /// The id needs to be unique within the same [`BinaryBenchmark`]
    ///
    /// # Examples
    ///
    /// The [`BenchmarkId`] can be created from any &str-like
    ///
    /// ```
    /// use iai_callgrind::Bench;
    ///
    /// let bench = Bench::new("my_unique_id");
    /// ```
    ///
    /// but you can also provide the [`BenchmarkId`]
    ///
    /// ```
    /// use iai_callgrind::{Bench, BenchmarkId};
    ///
    /// let bench = Bench::new(BenchmarkId::new("my_unique_id"));
    /// ```
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

    /// Add a [`BinaryBenchmarkConfig`] for this `Bench`
    ///
    /// A `Bench` without a `BinaryBenchmarkConfig` behaves like having specified the default
    /// [`BinaryBenchmarkConfig`]. This [`BinaryBenchmarkConfig`] overwrites the values of a
    /// [`BinaryBenchmarkConfig`] specified at a higher level. See there for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::{Bench, BinaryBenchmarkConfig};
    ///
    /// let bench = Bench::new("some_id").config(BinaryBenchmarkConfig::default().env("FOO", "BAR"));
    /// ```
    pub fn config<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalBinaryBenchmarkConfig>,
    {
        self.config = Some(config.into());
        self
    }

    /// Add a [`Command`] to this `Bench`
    ///
    /// A `Bench` with multiple `Commands` behaves exactly as the
    /// [`#[benches]`](crate::binary_benchmark) attribute
    ///
    /// # Errors
    ///
    /// It is an error if a `Bench` does not contain any `Commands`, so this method or
    /// [`Bench::commands`] has to be called at least once.
    ///
    /// # Examples
    ///
    /// Suppose the crate's binary is called `my-echo`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{Bench, Command};
    ///
    /// let bench = Bench::new("some_id")
    ///     .command(Command::new(env!("CARGO_BIN_EXE_my-echo")))
    ///     .clone();
    ///
    /// assert_eq!(bench.commands.len(), 1);
    /// ```
    pub fn command<T>(&mut self, command: T) -> &mut Self
    where
        T: Into<Command>,
    {
        self.commands.push(command.into());
        self
    }

    /// Add multiple [`Command`]s to this `Bench`
    ///
    /// See also [`Bench::command`].
    ///
    /// # Examples
    ///
    /// Suppose the crate's binary is called `my-echo`
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{Bench, Command};
    ///
    /// let mut command = Command::new(env!("CARGO_BIN_EXE_my-echo"));
    ///
    /// let echo_foo = command.clone().arg("foo").build();
    /// let echo_bar = command.arg("bar").build();
    ///
    /// let mut bench = Bench::new("some_id");
    /// bench.commands([echo_foo, echo_bar]).clone();
    ///
    /// assert_eq!(bench.commands.len(), 2);
    /// ```
    pub fn commands<I, T>(&mut self, commands: T) -> &mut Self
    where
        I: Into<Command>,
        T: IntoIterator<Item = I>,
    {
        self.commands.extend(commands.into_iter().map(Into::into));
        self
    }

    /// Add a `setup` function to be executed before the [`Command`] is executed
    ///
    /// This `setup` function overwrites the `setup` function of [`BinaryBenchmark`]. In the
    /// presence of a [`Sandbox`], this function is executed in the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bench;
    ///
    /// fn my_setup() {
    ///     println!("Place everything in this function you want to be executed prior to the Command");
    /// }
    ///
    /// let mut bench = Bench::new("some_id");
    /// bench.setup(my_setup);
    ///
    /// assert!(bench.setup.is_some())
    /// ```
    ///
    /// Overwrite the setup function from a [`BinaryBenchmark`]
    ///
    /// ```rust
    /// use iai_callgrind::{Bench, BinaryBenchmark};
    /// fn binary_benchmark_setup() {
    ///     println!("setup in BinaryBenchmark")
    /// }
    ///
    /// fn bench_setup() {
    ///     println!("setup in Bench")
    /// }
    ///
    /// BinaryBenchmark::new("bench_binary")
    ///     .setup(binary_benchmark_setup)
    ///     .bench(Bench::new("some_id").setup(bench_setup));
    /// ```
    pub fn setup(&mut self, setup: fn()) -> &mut Self {
        self.setup = Some(setup);
        self
    }

    /// Add a `teardown` function to be executed after the [`Command`] is executed
    ///
    /// This `teardown` function overwrites the `teardown` function of [`BinaryBenchmark`]. In the
    /// presence of a [`Sandbox`], this function is executed in the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bench;
    ///
    /// fn my_teardown() {
    ///     println!(
    ///         "Place everything in this function you want to be executed after the execution of the \
    ///          Command"
    ///     );
    /// }
    ///
    /// let mut bench = Bench::new("some_id");
    /// bench.teardown(my_teardown);
    ///
    /// assert!(bench.teardown.is_some())
    /// ```
    ///
    /// Overwrite the teardown function from a [`BinaryBenchmark`]
    ///
    /// ```rust
    /// use iai_callgrind::{Bench, BinaryBenchmark};
    /// fn binary_benchmark_teardown() {
    ///     println!("teardown in BinaryBenchmark")
    /// }
    ///
    /// fn bench_teardown() {
    ///     println!("teardown in Bench")
    /// }
    ///
    /// BinaryBenchmark::new("bench_binary")
    ///     .teardown(binary_benchmark_teardown)
    ///     .bench(Bench::new("some_id").teardown(bench_teardown));
    /// ```
    pub fn teardown(&mut self, teardown: fn()) -> &mut Self {
        self.teardown = Some(teardown);
        self
    }

    /// Add each line of a file as [`Command`] to this [`Bench`] using a `generator` function.
    ///
    /// This method mirrors the `file` parameter of the `#[benches]` attribute as far as possible.
    /// In the low-level api you can achieve the same or more quickly yourself and this method
    /// exists for the sake of completeness (and convenience).
    ///
    /// The file has to exist at the time you're using this method and the file has to be encoded in
    /// UTF-8. The `generator` function tells us how to convert each line of the file into a
    /// [`Command`].
    ///
    /// # Notable differences to high-level api
    ///
    /// If the file path in the high-level api is relative we interpret the path relative to the
    /// workspace root (and make it absolute). In this method we use the path AS IS.
    ///
    /// # Errors
    ///
    /// If the file is empty, cannot be opened for reading or a line in the file cannot be converted
    /// to a String. Also, the error from the `generator` is propagated. The errors containing the
    /// line number use a 0-indexed line number.
    ///
    /// # Examples
    ///
    /// Suppose your cargo's binary is named `my-echo` and you want to convert a file with inputs
    /// `benches/inputs` into commands and each line is the only argument for your `my-echo` binary:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{binary_benchmark_group, BinaryBenchmark, Bench};
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
    ///         group.binary_benchmark(BinaryBenchmark::new("some_id")
    ///             .bench(Bench::new("bench_id")
    ///                 .file("benches/inputs", |line| {
    ///                     Ok(iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-echo"))
    ///                         .arg(line)
    ///                         .build())
    ///                 }).unwrap()
    ///             )
    ///         )
    ///     }
    /// );
    /// # fn main() {}
    /// ```
    pub fn file<T>(
        &mut self,
        path: T,
        generator: fn(String) -> Result<Command, String>,
    ) -> Result<&mut Self, String>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(|error| {
            format!(
                "{}: Error opening file '{}': {error}",
                self.id,
                path.display(),
            )
        })?;

        let reader = BufReader::new(file);
        let mut has_lines = false;
        for (index, line) in reader.lines().enumerate() {
            has_lines = true;

            let line = line.map_err(|error| {
                format!(
                    "{}: Error reading line {index} in file '{}': {error}",
                    self.id,
                    path.display()
                )
            })?;

            let command = generator(line).map_err(|error| {
                format!(
                    "{}: Error generating command from line {index} in file '{}': {error}",
                    self.id,
                    path.display()
                )
            })?;
            self.commands.push(command);
        }

        if !has_lines {
            return Err(format!("{}: Empty file '{}'", self.id, path.display()));
        }
        Ok(self)
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
    /// Convenience method to create a `BenchmarkId` with a parameter in the [low level
    /// api](crate::binary_benchmark_group)
    ///
    /// The `parameter` is simply appended to the `id` with an underscore, so
    /// `BenchmarkId::with_parameter("some", 1)` is equivalent to `BenchmarkId::new("some_1")`
    ///
    /// # Examples
    ///
    /// ```
    /// use iai_callgrind::BenchmarkId;
    ///
    /// let new_id = BenchmarkId::new("prefix_1");
    /// let with_parameter = BenchmarkId::with_parameter("prefix", 1);
    /// assert_eq!(new_id, with_parameter);
    /// ```
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group,BenchmarkId, BinaryBenchmark, Bench, Command};
    /// use std::ffi::OsStr;
    ///
    /// binary_benchmark_group!(
    ///     name = low_level_group;
    ///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
    ///         let mut binary_benchmark = BinaryBenchmark::new("some_id");
    ///         for arg in 0..10 {
    ///             let id = BenchmarkId::with_parameter("prefix", arg);
    ///             binary_benchmark.bench(
    ///                 Bench::new(id)
    ///                     .command(
    ///                         Command::new("echo").arg(arg.to_string()).build()
    ///                     )
    ///             );
    ///         }
    ///         group.binary_benchmark(binary_benchmark);
    ///     }
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = low_level_group);
    /// # }
    /// ```
    pub fn with_parameter<T, P>(id: T, parameter: P) -> Self
    where
        T: AsRef<str>,
        P: Display,
    {
        Self(format!("{}_{parameter}", id.as_ref()))
    }

    /// Create a new `BenchmarkId`
    ///
    /// `BenchmarkId`s can be created from any string-like input. See [`BenchmarkId::validate`] for
    /// ids which are considered valid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::BenchmarkId;
    ///
    /// let id = BenchmarkId::new("my_id");
    ///
    /// assert!(id.validate().is_ok());
    /// ```
    pub fn new<T>(id: T) -> Self
    where
        T: Into<String>,
    {
        Self(id.into())
    }

    /// Returns ok if this [`BenchmarkId`] is valid
    ///
    /// An id should be short, descriptive besides being unique. The requirements for the uniqueness
    /// differ for the structs where a `BenchmarkId` is used and is further described there.
    ///
    /// We use a minimal subset of rust's identifiers. A valid `BenchmarkId` starts with an ascii
    /// alphabetic letter [a-zA-Z] or underscore [_]. All following characters can be an ascii
    /// alphabetic letter, underscore or a digit [0-9]. At least one valid character must be
    /// present.
    ///
    /// The `BenchmarkId` is used by `iai-callgrind` as file and directory name for the output files
    /// of the benchmarks. Therefore, the limit for an id is chosen to be 120 bytes. This is a
    /// calculation with some headroom. There are file systems which do not even allow 255 bytes. If
    /// you're working on such a peculiar file system, you have to restrict your ids to even fewer
    /// bytes which is `floor(MAX_LENGTH/2) - 1`. Leaving the maximum bytes aside, the best IDs are
    /// simple, short and descriptive.
    ///
    /// Usually, it is not necessary to call this function, since we already check the validity of
    /// benchmark ids prior to the execution of the benchmark runner. But if your ids come from an
    /// untrusted source you can use this method for more immediate feedback.
    ///
    /// # Errors
    ///
    /// This function will return an error describing the source of the error if the id is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::BenchmarkId;
    ///
    /// assert!(BenchmarkId::new("").validate().is_err());
    /// assert!(BenchmarkId::new("0a").validate().is_err());
    /// assert!(BenchmarkId::new("id with spaces").validate().is_err());
    /// assert!(BenchmarkId::new("path/to").validate().is_err());
    /// assert!(BenchmarkId::new("no::module::too").validate().is_err());
    ///
    /// assert!(BenchmarkId::new("_").validate().is_ok());
    /// assert!(BenchmarkId::new("abc").validate().is_ok());
    /// assert!(BenchmarkId::new("a9").validate().is_ok());
    /// assert!(BenchmarkId::new("z_").validate().is_ok());
    /// assert!(BenchmarkId::new("some_id").validate().is_ok());
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn validate(&self) -> Result<(), String> {
        const MAX_LENGTH_ID: usize = 255;
        if self.0.is_empty() {
            return Err("Invalid id: Cannot be empty".to_owned());
        }

        let mut bytes = self.0.bytes();
        // This unwrap is safe, since we just checked that the string is not empty
        let first = bytes.next().unwrap();

        if first.is_ascii_alphabetic() || first == b'_' {
            for (index, byte) in (1..).zip(bytes) {
                if index > MAX_LENGTH_ID {
                    return Err(format!(
                        "Invalid id '{}': Maximum length of {MAX_LENGTH_ID} bytes reached",
                        &self.0,
                    ));
                }
                if byte.is_ascii() {
                    if !(byte.is_ascii_alphanumeric() || byte == b'_') {
                        return Err(format!(
                            "Invalid id '{}' at position {index}: Invalid character '{}'",
                            &self.0,
                            char::from(byte)
                        ));
                    }
                } else {
                    return Err(format!(
                        "Invalid id '{}' at position {index}: Encountered non-ascii character",
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

impl From<BenchmarkId> for String {
    fn from(value: BenchmarkId) -> Self {
        value.0
    }
}

impl<T> From<T> for BenchmarkId
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        Self(value.as_ref().to_owned())
    }
}

impl BinaryBenchmark {
    /// Create a new `BinaryBenchmark`
    ///
    /// A `BinaryBenchmark` is the equivalent of the
    /// [`#[binary_benchmark]`](`crate::binary_benchmark`) attribute in the low-level api and needs
    /// a [`BenchmarkId`]. In the high-level api the id is derived from the function name.
    ///
    /// The [`BenchmarkId`] is used together with the id of each [`Bench`] to create a directory
    /// name. This usually limits the combined length of the ids to `255 - 1` but can be less
    /// depending on the file system. See [`BenchmarkId`] for more details
    ///
    /// # Scope of uniqueness of the [`BenchmarkId`]
    ///
    /// The id needs to be unique within the same [`crate::binary_benchmark_group`]. It is
    /// recommended to use an id which is unique within the whole benchmark file, although doing
    /// otherwise does currently not incur any negative consequence.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{BenchmarkId, BinaryBenchmark};
    ///
    /// let binary_benchmark = BinaryBenchmark::new("some_id");
    /// assert_eq!(binary_benchmark.id, BenchmarkId::new("some_id"));
    /// ```
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

    /// Add a [`BinaryBenchmarkConfig`] to this `BinaryBenchmark`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{BinaryBenchmark, BinaryBenchmarkConfig};
    ///
    /// let binary_benchmark = BinaryBenchmark::new("some_id")
    ///     .config(BinaryBenchmarkConfig::default().env("FOO", "BAR"))
    ///     .clone();
    ///
    /// assert_eq!(
    ///     binary_benchmark.config,
    ///     Some(BinaryBenchmarkConfig::default().env("FOO", "BAR").into())
    /// );
    /// ```
    pub fn config<T>(&mut self, config: T) -> &mut Self
    where
        T: Into<internal::InternalBinaryBenchmarkConfig>,
    {
        self.config = Some(config.into());
        self
    }

    /// Add a [`Bench`] to this `BinaryBenchmark`
    ///
    /// Adding a [`Bench`] which doesn't contain a [`Command`] is an error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{Bench, BinaryBenchmark, Command};
    ///
    /// // Each `Bench` needs at least one `Command`!
    /// let mut bench = Bench::new("some_id");
    /// bench.command(Command::new(env!("CARGO_BIN_EXE_my-echo")));
    ///
    /// let binary_benchmark = BinaryBenchmark::new("bench_binary")
    ///     .bench(bench.clone())
    ///     .clone();
    ///
    /// assert_eq!(binary_benchmark.benches[0], bench);
    /// ```
    pub fn bench<T>(&mut self, bench: T) -> &mut Self
    where
        T: Into<Bench>,
    {
        self.benches.push(bench.into());
        self
    }

    /// Add a `setup` function to this `BinaryBenchmark`
    ///
    /// This `setup` function is used in all [`Bench`]es of this `BinaryBenchmark` if not overridden
    /// by the `Bench`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bench;
    /// fn my_setup() {
    ///     println!(
    ///         "Place everything in this function you want to be executed before the execution of \
    ///          the Command"
    ///     );
    /// }
    ///
    /// let bench = Bench::new("some_id").setup(my_setup).clone();
    ///
    /// assert!(bench.setup.is_some())
    /// ```
    ///
    /// Overwrite the setup function from this `BinaryBenchmark` in a [`Bench`]
    ///
    /// ```rust
    /// use iai_callgrind::{Bench, BinaryBenchmark};
    /// fn binary_benchmark_setup() {
    ///     println!("setup in BinaryBenchmark")
    /// }
    ///
    /// fn bench_setup() {
    ///     println!("setup in Bench")
    /// }
    ///
    /// BinaryBenchmark::new("bench_binary")
    ///     .setup(binary_benchmark_setup)
    ///     .bench(Bench::new("some_id").setup(bench_setup));
    /// ```
    pub fn setup(&mut self, setup: fn()) -> &mut Self {
        self.setup = Some(setup);
        self
    }

    /// Add a `teardown` function to this `BinaryBenchmark`
    ///
    /// This `teardown` function is used in all [`Bench`]es of this `BinaryBenchmark` if not
    /// overridden by the `Bench`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Bench;
    /// fn my_teardown() {
    ///     println!(
    ///         "Place everything in this function you want to be executed after the execution of the \
    ///          Command"
    ///     );
    /// }
    ///
    /// let bench = Bench::new("some_id").teardown(my_teardown).clone();
    ///
    /// assert!(bench.teardown.is_some())
    /// ```
    ///
    /// Overwrite the teardown function from this `BinaryBenchmark` in a [`Bench`]
    ///
    /// ```rust
    /// use iai_callgrind::{Bench, BinaryBenchmark};
    /// fn binary_benchmark_teardown() {
    ///     println!("teardown in BinaryBenchmark")
    /// }
    ///
    /// fn bench_teardown() {
    ///     println!("teardown in Bench")
    /// }
    ///
    /// BinaryBenchmark::new("bench_binary")
    ///     .teardown(binary_benchmark_teardown)
    ///     .bench(Bench::new("some_id").teardown(bench_teardown));
    /// ```
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
    /// BinaryBenchmarkConfig::default()
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

    /// Add an environment variable to the [`Command`]
    ///
    /// These environment variables are available independently of the setting of
    /// [`BinaryBenchmarkConfig::env_clear`].
    ///
    /// # Examples
    ///
    /// An example for a custom environment variable "FOO=BAR":
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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

    /// Add multiple environment variables to the [`Command`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// See also [`crate::BinaryBenchmarkConfig::pass_through_env`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::binary_benchmark_group;
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// In the case of running without sandboxing enabled, this'll be the directory which `cargo
    /// bench` sets. If running the benchmark within the sandbox, and the path is relative then this
    /// new directory must be contained in the sandbox.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, Sandbox};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .sandbox(Sandbox::new(true))
    ///         .current_dir("fixtures");
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
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
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{
    ///     binary_benchmark, binary_benchmark_group, BinaryBenchmarkConfig, main, Tool,
    ///     ValgrindTool
    /// };
    ///
    /// #[binary_benchmark]
    /// #[bench::some(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tool_override(Tool::new(ValgrindTool::Memcheck))
    /// )]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
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
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{
    ///     binary_benchmark, binary_benchmark_group, BinaryBenchmarkConfig, main, Tool,
    ///     ValgrindTool
    /// };
    ///
    /// #[binary_benchmark]
    /// #[bench::some(
    ///     config = BinaryBenchmarkConfig::default()
    ///         .tools_override([
    ///             Tool::new(ValgrindTool::Massif),
    ///             Tool::new(ValgrindTool::Memcheck),
    ///         ])
    /// )]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
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

    /// Configure benchmarks to run in a [`Sandbox`] (Default: false)
    ///
    /// If specified, we create a temporary directory in which the `setup` and `teardown` functions
    /// of the `#[binary_benchmark]` (`#[bench]`, `#[benches]`) and the [`Command`] itself are run.
    ///
    /// A good reason for using a temporary directory as workspace is, that the length of the path
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
    /// # use iai_callgrind::{binary_benchmark_group};
    /// # binary_benchmark_group!(
    /// #    name = my_group;
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
    /// use iai_callgrind::{main, BinaryBenchmarkConfig, Sandbox};
    ///
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true));
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
    /// #    benchmarks = |_group: &mut BinaryBenchmarkGroup| {}
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
    /// Add a [`BinaryBenchmark`] to this group
    ///
    /// This can be a binary benchmark created with [`BinaryBenchmark::new`] or a
    /// [`crate::binary_benchmark`] attributed function addable with the
    /// [`crate::binary_benchmark_attribute`] macro.
    ///
    /// It is an error to add a [`BinaryBenchmark`] without having added a [`Bench`] to it.
    ///
    /// # Examples
    ///
    /// Add a [`BinaryBenchmark`] to this group
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{binary_benchmark_group, BinaryBenchmark, Bench, BinaryBenchmarkGroup};
    ///
    /// fn setup_my_group(group: &mut BinaryBenchmarkGroup) {
    ///     group.binary_benchmark(BinaryBenchmark::new("bench_binary")
    ///         .bench(Bench::new("foo")
    ///             .command(iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
    ///                 .arg("foo")
    ///             )
    ///         )
    ///     );
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = |group: &mut BinaryBenchmarkGroup| setup_my_group(group)
    /// );
    /// # fn main() {}
    /// ```
    ///
    /// Or, add a `#[binary_benchmark]` annotated function to this group:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{binary_benchmark, binary_benchmark_group, binary_benchmark_attribute};
    ///
    /// #[binary_benchmark]
    /// #[bench::foo("foo")]
    /// fn bench_binary(arg: &str) -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
    ///         .arg(arg)
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
    ///         group
    ///             .binary_benchmark(binary_benchmark_attribute!(bench_binary))
    ///     }
    /// );
    /// # fn main() {}
    /// ```
    pub fn binary_benchmark<T>(&mut self, binary_benchmark: T) -> &mut Self
    where
        T: Into<BinaryBenchmark>,
    {
        self.binary_benchmarks.push(binary_benchmark.into());
        self
    }

    /// Add multiple [`BinaryBenchmark`]s at once
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

/// Provide the [`Command`] to be benchmarked
///
/// This is a builder for [`std::process::Command`]. As opposed to [`std::process::Command`] there
/// is no option to execute this command immediately. Instead, we collect all commands in this
/// benchmark file and execute them later.
///
/// The default configuration is created with [`Command::new`]. The builder methods allow the
/// configuration to be changed prior to [`Command::build`]. The [`Command`] can be reused to build
/// multiple processes. However, this is only possible with the [low level
/// api](crate::binary_benchmark_group)
impl Command {
    /// Create a new [`Command`] which is run under valgrind.
    ///
    /// Use
    /// [`env!("CARGO_BIN_EXE_<name>)`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)
    /// to provide the path to an executable of your project instead of `target/release/<name>`.
    /// This command is a builder for a [`std::process::Command`] and is not executed right away. We
    /// simply gather all the information to be finally able to execute the command under valgrind,
    /// later (after we collected all the commands in this benchmark file). As opposed to
    /// [`std::process::Command`], the build is finalized with [`Command::build`].
    ///
    /// # Relative paths
    ///
    /// Even though relative paths are discouraged, they are always interpreted relative to the
    /// workspace root as reported by cargo. This is usually the directory with the top-level
    /// `Cargo.toml` file. We try to resolve simple names like `Command::new("echo")` using the
    /// `$PATH`. To disambiguate between simple names and relative paths, use `./` as prefix for
    /// relative paths. For example `echo` is searched in the `$PATH` and `./echo` is interpreted as
    /// relative to the workspace root.
    ///
    /// # Examples
    ///
    /// Assume the, or one of your project's binaries name is `my-echo`:
    ///
    /// ```
    /// # macro_rules! env { ($m:tt) => {{ "/home/my_project/target/release/my-echo" }} }
    /// use iai_callgrind::Command;
    ///
    /// let command = Command::new(env!("CARGO_BIN_EXE_my-echo"));
    /// ```
    ///
    /// or use your system's echo from the `$PATH` with
    ///
    /// ```
    /// use iai_callgrind::Command;
    ///
    /// let command = Command::new("echo").arg("foo").build();
    /// ```
    pub fn new<T>(path: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        Self(internal::InternalCommand {
            path: PathBuf::from(path.as_ref()),
            ..Default::default()
        })
    }

    /// Adds an argument to pass to the [`Command`]
    ///
    /// This option works exactly the same way as [`std::process::Command::arg`]. To pass multiple
    /// arguments see [`Command::args`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use iai_callgrind::main;
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-echo"))
    ///         .arg("foo")
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn arg<T>(&mut self, arg: T) -> &mut Self
    where
        T: Into<OsString>,
    {
        self.0.args.push(arg.into());
        self
    }

    /// Adds multiple arguments to pass to the [`Command`]
    ///
    /// This option works exactly the same way as [`std::process::Command::args`].
    ///
    /// # Examples
    ///
    /// The following will execute `my-echo foo`.
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-echo"))
    ///         .arg("foo")
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn args<I, T>(&mut self, args: T) -> &mut Self
    where
        I: Into<OsString>,
        T: IntoIterator<Item = I>,
    {
        self.0.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Configuration for the process's standard input ([`Stdin`])
    ///
    /// This method takes an [`Stdin`], [`Stdio`] and everything that implements `Into<Stdin>`. The
    /// [`Stdin`] enum mirrors most of the possibilities of [`std::process::Stdio`] but also some
    /// additional possibilities most notably [`Stdin::Setup`] (see there for more details).
    ///
    /// Per default, the stdin is not inherited from the parent and any attempt by the child process
    /// to read from the stdin stream will result in the stream immediately closing.
    ///
    /// The options you might be interested in the most are [`Stdin::File`], which mirrors the
    /// behaviour of [`std::process::Stdio`] if `Stdio` is a [`std::fs::File`], and
    /// [`Stdin::Setup`], which is special to `iai-callgrind` and let's you pipe the output of
    /// the `setup` function into the Stdin of this [`Command`].
    ///
    /// # Implementation details
    ///
    /// As the [`Command`] itself is not executed immediately, the [`std::process::Stdio`] is not
    /// created either. We only use the information from here to create the [`std::process::Stdio`]
    /// later after we collected all commands. Setting the Stdin to `Inherit` is discouraged and
    /// won't have the effect you might expect, since the benchmark runner (the parent) uses the
    /// Stdin for its own purposes and closes it before this [`Command`] is executed.
    ///
    /// # Examples
    ///
    /// Pipe the content of a file (`benches/fixture.txt`) into the stdin of this [`Command`]:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdio};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stdin(Stdio::File("benches/fixture.txt".into()))
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// Pipe the Stdout of setup into the Stdin of this [`Command`]:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdin, Pipe};
    ///
    /// fn setup_pipe() {
    ///     // All output to Stdout of this function will be piped into the Stdin of `my-exe`
    ///     println!("This string will be piped into the stdin of my-exe");
    /// }
    ///
    /// #[binary_benchmark(setup = setup_pipe())]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stdin(Stdin::Setup(Pipe::Stdout))
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    pub fn stdin<T>(&mut self, stdin: T) -> &mut Self
    where
        T: Into<Stdin>,
    {
        self.0.stdin = Some(stdin.into());
        self
    }

    /// Configuration for the [`Command`]s standard output (Stdout) handle.
    ///
    /// The output of benchmark commands and functions are usually captured by the benchmark runner.
    /// This can be changed for example with the `--nocapture` option or here. Any option specified
    /// here takes precedence over the changes which `--nocapture` makes to the Stdout of the
    /// command.
    ///
    /// # Examples
    ///
    /// To see the output of this [`Command`] regardless of `--nocapture` in the benchmark output
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdio};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stdout(Stdio::Inherit)
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// To pipe the Stdout into a file `/tmp/benchmark.output`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdio};
    /// use std::path::PathBuf;
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stdout(Stdio::File("/tmp/benchmark.output".into()))
    ///         // or
    ///         .stdout(PathBuf::from("/tmp/benchmark.output"))
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn stdout<T>(&mut self, stdio: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.0.stdout = Some(stdio.into());
        self
    }

    /// Configuration for the [`Command`]s standard error output (Stderr) handle.
    ///
    /// This option is similar to [`Command::stdout`] but configures the Stderr. See there for more
    /// details.
    ///
    /// # Examples
    ///
    /// To see the error output of this [`Command`] regardless of `--nocapture` in the benchmark
    /// output
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdio};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stderr(Stdio::Inherit)
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// To pipe the Stderr into a file `/tmp/benchmark.output`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Stdio};
    /// use std::path::PathBuf;
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .stderr(Stdio::File("/tmp/benchmark.output".into()))
    ///         // or
    ///         .stderr(PathBuf::from("/tmp/benchmark.output"))
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn stderr<T>(&mut self, stdio: T) -> &mut Self
    where
        T: Into<Stdio>,
    {
        self.0.stderr = Some(stdio.into());
        self
    }

    /// Add an environment variable available for this [`Command`]
    ///
    /// These environment variables are available independently of the setting of
    /// [`BinaryBenchmarkConfig::env_clear`] and additive to environment variables added with
    /// [`BinaryBenchmarkConfig::env`].
    ///
    /// # Examples
    ///
    /// An example for a custom environment variable "FOO=BAR":
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .env("FOO", "BAR")
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
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

    /// Add multiple environment variables available for this [`Command`]
    ///
    /// See [`Command::env`] for more details.
    ///
    /// # Examples
    ///
    /// Add the custom environment variables "FOO=BAR" and `BAR=BAZ`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .envs([("FOO", "BAR"), ("BAR", "BAZ")])
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
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

    /// Set the directory of the benchmarked binary (Default: Unchanged)
    ///
    /// See also [`BinaryBenchmarkConfig::current_dir`]
    ///
    /// # Examples
    ///
    /// To set the working directory of your [`Command`] to `/tmp`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .current_dir("/tmp")
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// and the following will change the current directory to `fixtures` located in the root of the
    /// [`BinaryBenchmarkConfig::sandbox`]
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, Sandbox, BinaryBenchmarkConfig};
    ///
    /// fn setup_sandbox() {
    ///     std::fs::create_dir("fixtures").unwrap();
    /// }
    ///
    /// #[binary_benchmark(
    ///     setup = setup_sandbox(),
    ///     config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true))
    /// )]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .current_dir("fixtures")
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
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

    /// Set the expected exit status [`ExitWith`] of this [`Command`]
    ///
    /// See also [`BinaryBenchmarkConfig::exit_with`]. This setting overwrites the setting of the
    /// [`BinaryBenchmarkConfig`].
    ///
    /// # Examples
    ///
    /// If the command is expected to exit with a specific code, for example `100`:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, ExitWith};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .exit_with(ExitWith::Code(100))
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    ///
    /// If a command is expected to fail, but the exit code doesn't matter:
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// # use iai_callgrind::main;
    /// use iai_callgrind::{binary_benchmark_group, binary_benchmark, ExitWith};
    ///
    /// #[binary_benchmark]
    /// fn bench_binary() -> iai_callgrind::Command {
    ///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///         .exit_with(ExitWith::Failure)
    ///         .build()
    /// }
    ///
    /// binary_benchmark_group!(
    ///     name = my_group;
    ///     benchmarks = bench_binary
    /// );
    /// # fn main() {
    /// # main!(binary_benchmark_groups = my_group);
    /// # }
    /// ```
    pub fn exit_with(&mut self, exit_with: ExitWith) -> &mut Self {
        self.0.config.exit_with = Some(exit_with.into());
        self
    }

    /// Finalize and build this [`Command`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
    /// use iai_callgrind::Command;
    ///
    /// let command: Command = Command::new(env!("CARGO_BIN_EXE_my-exe"))
    ///     .arg("foo")
    ///     .build();
    /// ```
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

// TODO: ADD `follow_symlinks` and maybe others. See `InternalSandbox`
// TODO: ADD `copy_back`
// TODO: ADD `chroot`
/// The `Sandbox` in which the the `setup`, `teardown` and the [`Command`] are run
///
/// The `Sandbox` is a temporary directory which is created before the execution of the
/// [`setup`](`crate::binary_benchmark`) and deleted after the
/// [`teardown`](`crate::binary_benchmark`). `setup`, the [`Command`] and `teardown` are executed
/// inside this temporary directory.
///
/// # Background and reasons for using a `Sandbox`
///
/// A [`Sandbox`] can help mitigating differences in benchmark results on different machines. As
/// long as `$TMP_DIR` is unset or set to `/tmp`, the temporary directory has a constant length on
/// unix machines (with the exception of android which uses `/data/local/tmp`). The directory itself
/// is created with a constant length but random name like `/tmp/.a23sr8fk`. It is not implausible
/// that an executable has different event counts just because the directory it is executed in has a
/// different length. For example, if a member of your project has set up the project in
/// `/home/bob/workspace/our-project` running the benchmarks in this directory, and the ci runs the
/// benchmarks in `/runner/our-project`, the event counts might differ. If possible, the benchmarks
/// should be run in a as constant as possible environment. Clearing the environment variables is
/// also such a counter-measure.
///
/// Other reasons for using a `Sandbox` are convenience, such as if you're creating files during
/// `setup` and the [`Command`] run and don't want to delete all the files manually. Or, more
/// importantly, if the [`Command`] is destructive and deletes files, it is usually safer to execute
/// such a [`Command`] in a temporary directory where it cannot do any harm to your or others file
/// systems during the benchmark runs.
///
/// # Sandbox cleanup
///
/// The changes the `setup` makes in this directory persist until the `teardown` has finished. So,
/// the [`Command`] can for example pick up any files created by the `setup` method. If run in a
/// `Sandbox`, the `teardown` usually doesn't have to delete any files, because the whole
/// directory is deleted after its usage. There is an exception to the rule. If any of the files
/// inside the directory is not removable, for example because the permissions of a file don't allow
/// the file to be deleted, then the whole directory persists. You can use the `teardown` to reset
/// all permission bits to be readable and writable, so the cleanup can succeed.
///
/// To simply copy fixtures or whole directories into the `Sandbox` use [`Sandbox::fixtures`].
impl Sandbox {
    /// Create a new `Sandbox` builder
    ///
    /// Per default, a [`Command`] is not run in a `Sandbox` because setting up a `Sandbox` usually
    /// involves some user interaction, for example copying fixtures into it with
    /// [`Sandbox::fixtures`].
    ///
    /// The temporary directory is only created immediately before the `setup` and the [`Command`]
    /// are executed.
    ///
    /// # Examples
    ///
    /// Enable the sandbox for all benchmarks
    ///
    /// ```rust
    /// use iai_callgrind::{BinaryBenchmarkConfig, Sandbox, main};
    /// # use iai_callgrind::binary_benchmark_group;
    /// # binary_benchmark_group!(name = my_group; benchmarks = |_group| {});
    /// # fn main() {
    /// main!(
    ///     config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true));
    ///     binary_benchmark_groups = my_group
    /// );
    /// # }
    /// ```
    pub fn new(enabled: bool) -> Self {
        Self(internal::InternalSandbox {
            enabled: Some(enabled),
            ..Default::default()
        })
    }

    /// Specify the directories and/or files you want to copy into the root of the `Sandbox`
    ///
    /// The paths are interpreted relative to the workspace root as it is reported by `cargo`. In a
    /// multi-crate project this is the directory with the top-level `Cargo.toml`. Otherwise, it is
    /// simply the directory with your `Cargo.toml` file in it.
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
    #[case::multibyte_middle(
        "aµ",
        "Invalid id 'aµ' at position 1: Encountered non-ascii character"
    )]
    #[case::non_ascii_middle("a-", "Invalid id 'a-' at position 1: Invalid character '-'")]
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
