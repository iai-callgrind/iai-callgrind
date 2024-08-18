//! This module is only used for internal purposes and does not contain any publicly usable
//! structs
#![allow(missing_docs)]

pub use iai_callgrind_runner::api::{
    BinaryBenchmark as InternalBinaryBenchmark,
    BinaryBenchmarkBench as InternalBinaryBenchmarkBench,
    BinaryBenchmarkConfig as InternalBinaryBenchmarkConfig,
    BinaryBenchmarkGroup as InternalBinaryBenchmarkGroup,
    BinaryBenchmarkGroups as InternalBinaryBenchmarkGroups, Command as InternalCommand,
    ExitWith as InternalExitWith, Fixtures as InternalFixtures,
    FlamegraphConfig as InternalFlamegraphConfig,
    LibraryBenchmark as InternalLibraryBenchmarkBenches,
    LibraryBenchmarkBench as InternalLibraryBenchmarkBench,
    LibraryBenchmarkConfig as InternalLibraryBenchmarkConfig,
    LibraryBenchmarkGroup as InternalLibraryBenchmarkGroup,
    LibraryBenchmarkGroups as InternalLibraryBenchmarkGroups, RawArgs as InternalRawArgs,
    RegressionConfig as InternalRegressionConfig, Sandbox as InternalSandbox, Tool as InternalTool,
    Tools as InternalTools,
};

#[derive(Debug, Clone)]
pub struct InternalMacroLibBench {
    pub id_display: Option<&'static str>,
    pub args_display: Option<&'static str>,
    pub func: fn(),
    pub config: Option<fn() -> crate::internal::InternalLibraryBenchmarkConfig>,
}

#[derive(Debug, Clone)]
pub struct InternalMacroBinBench {
    pub id_display: Option<&'static str>,
    pub args_display: Option<&'static str>,
    pub func: fn() -> crate::Command,
    pub setup: Option<fn()>,
    pub teardown: Option<fn()>,
    pub config: Option<fn() -> crate::internal::InternalBinaryBenchmarkConfig>,
}
