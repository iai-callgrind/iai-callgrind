//! This module is only used for internal purposes and does not contain any publicly usable
//! structs
#![allow(missing_docs)]

pub mod bin_bench;
pub mod error;

// The runner api is not used directly in order to decouple the user interface and
// documentation from the internal usage.
//
// We re-export all structs with the `Internal` prefix to avoid accidental usage. The wrapper
// structs provided by the iai-callgrind module (in `iai_callgrind::bin_bench`, ...) are the
// structs to be used by the iai-callgrind end-user. Almost all of these structs use the
// builder pattern to build the api internal structures. The documentation visible to the user
// can be found in these builders.
//
// As an exception, enums from the runner api are usually used directly and re-exported in
// `lib.rs`.
pub use iai_callgrind_runner::api::{
    BinaryBenchmark as InternalBinaryBenchmark,
    BinaryBenchmarkBench as InternalBinaryBenchmarkBench,
    BinaryBenchmarkConfig as InternalBinaryBenchmarkConfig,
    BinaryBenchmarkGroup as InternalBinaryBenchmarkGroup,
    BinaryBenchmarkGroups as InternalBinaryBenchmarkGroups, Command as InternalCommand,
    Delay as InternalDelay, ExitWith as InternalExitWith, Fixtures as InternalFixtures,
    FlamegraphConfig as InternalFlamegraphConfig,
    LibraryBenchmark as InternalLibraryBenchmarkBenches,
    LibraryBenchmarkBench as InternalLibraryBenchmarkBench,
    LibraryBenchmarkConfig as InternalLibraryBenchmarkConfig,
    LibraryBenchmarkGroup as InternalLibraryBenchmarkGroup,
    LibraryBenchmarkGroups as InternalLibraryBenchmarkGroups, OutputFormat as InternalOutputFormat,
    RawArgs as InternalRawArgs, RegressionConfig as InternalRegressionConfig,
    Sandbox as InternalSandbox, Tool as InternalTool, Tools as InternalTools,
};

/// Used in iai-callgrind-macros to store the essential information about a library benchmark
#[derive(Debug, Clone)]
pub struct InternalMacroLibBench {
    pub id_display: Option<&'static str>,
    pub args_display: Option<&'static str>,
    pub func: fn(),
    pub config: Option<fn() -> InternalLibraryBenchmarkConfig>,
}

/// Used in iai-callgrind-macros to store the essential information about a binary benchmark
#[derive(Debug, Clone)]
pub struct InternalMacroBinBench {
    pub id_display: Option<&'static str>,
    pub args_display: Option<&'static str>,
    pub func: fn() -> crate::Command,
    pub setup: Option<fn()>,
    pub teardown: Option<fn()>,
    pub config: Option<fn() -> InternalBinaryBenchmarkConfig>,
}

/// A small internal helper to easily create module paths like `file::group::benchmark::id`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModulePath(String);

impl ModulePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn join(&self, path: &str) -> Self {
        Self(format!("{}::{path}", self.0))
    }
}

impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
