//! Iai-Callgrind is a high-precision and consistent benchmarking framework/harness which uses
//! [Valgrind's Callgrind](https://valgrind.org/docs/manual/cl-manual.html) to provide extremely
//! accurate and consistent measurements of Rust code, making it perfectly suited to run in
//! environments like a CI.
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
//! `iai-callgrind` can be used to benchmark libraries or binaries. Library benchmarks benchmark
//! functions and methods of a crate and binary benchmarks benchmark the executables of a crate. The
//! different benchmark types cannot be intermixed in the same benchmark file but having different
//! benchmark files for library and binary benchmarks is no problem.
//!
//! For a full description of the [`main`](crate::main) macro and possibilities of `iai-callgrind`
//! see the [README](https://github.com/Joining7943/iai-callgrind) of this crate.

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

pub use {bincode, serde};

pub mod internal;
mod macros;

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
///        args = ["file does not exist"];
/// );
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExitWith {
    /// Exit with success is similar to `ExitCode(0)`
    Success,
    /// Exit with failure is similar to setting the `ExitCode` to something different than `0`
    Failure,
    /// The exact `ExitCode` of the benchmark run
    Code(i32),
}

/// The `Options`, applied to each benchmark `run` of a benchmarked binary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Options {
    /// If false, don't clear the environment variables of a benchmarked binary (Default: true)
    pub env_clear: bool,
    /// Set the current directory of the benchmarked binary. (Default: Unchanged)
    pub current_dir: Option<PathBuf>,
    /// Set the entry point for event counting of the benchmarked binary
    pub entry_point: Option<String>,
    /// Set [`ExitWith`] to the expected exit status of the benchmarked binary
    pub exit_with: Option<ExitWith>,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    /// Create a new `Options` struct
    pub fn new() -> Self {
        Self {
            env_clear: true,
            current_dir: None,
            entry_point: None,
            exit_with: None,
        }
    }

    /// If false, don't clear the environment variables before running the benchmark (Default: true)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    ///
    /// iai_callgrind::main!(
    ///     run = cmd = "my_exe",
    ///     opts = Options::default().env_clear(false),
    ///     args = []
    /// );
    /// ```
    pub fn env_clear(mut self, value: bool) -> Self {
        self.env_clear = value;
        self
    }

    /// Set the directory of the benchmarked binary (Default: Unchanged)
    ///
    /// If running the benchmark within the sandbox, and the path is relative then this new
    /// directory must be contained in the sandbox.
    ///
    /// # Examples
    ///
    /// This'll change the current directory of the `my_exe` binary to `/tmp`
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    /// use std::path::PathBuf;
    ///
    /// iai_callgrind::main!(
    ///    run = cmd = "my_exe",
    ///        opts = Options::default().current_dir(PathBuf::from("/tmp")),
    ///        args = [];
    /// );
    /// ```
    ///
    /// and the following will change the current directory of `my_exe` to `fixtures` which is
    /// contained in the sandbox
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    /// use std::path::PathBuf;
    ///
    /// iai_callgrind::main!(
    ///    fixtures = "benches/fixtures";
    ///    run = cmd = "my_exe",
    ///        opts = Options::default().current_dir(PathBuf::from("fixtures")),
    ///        args = [];
    /// );
    /// ```
    pub fn current_dir(mut self, value: PathBuf) -> Self {
        self.current_dir = Some(value);
        self
    }

    /// Set the entry point for event counting of the binary
    ///
    /// Per default, the counting of events starts right at the start of the binary and stops when
    /// it finished execution. It may desirable to start the counting for example when entering
    /// the main function (but can be any function) and stop counting when leaving the main
    /// function of the executable.
    ///
    /// # Examples
    ///
    /// The `entry_point` could look like `my_exe::main` for a binary with the name `my-exe` (Note
    /// that hyphens are replaced with an underscore).
    ///
    /// ```rust
    /// use iai_callgrind::Options;
    ///
    /// iai_callgrind::main!(
    ///    run = cmd = "my-exe",
    ///        opts = Options::default().entry_point("my_exe::main"),
    ///        args = [];
    /// );
    /// ```
    pub fn entry_point(mut self, value: &str) -> Self {
        self.entry_point = Some(value.to_owned());
        self
    }

    /// Set the expected exit status [`ExitWith`] of a benchmarked binary
    ///
    /// Per default, the benchmarked binary is expected to succeed, but if a benchmark is expected
    /// to fail, setting this option is required.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use iai_callgrind::{Options, ExitWith};
    ///
    /// iai_callgrind::main!(
    ///    run = cmd = "/bin/stat",
    ///        opts = Options::default().exit_with(ExitWith::Code(1)),
    ///        args = ["file does not exist"];
    /// );
    /// ```
    pub fn exit_with(mut self, value: ExitWith) -> Self {
        self.exit_with = Some(value);
        self
    }
}
