//! This module is only used for internal purposes and does not contain any publicly usable
//! structs
#![allow(missing_docs)]

pub use iai_callgrind_runner::api::{
    Arg as RunnerArg, Assistant as RunnerAssistant, BinaryBenchmark as RunnerBinaryBenchmark,
    BinaryBenchmarkGroup as RunnerBinaryBenchmarkGroup, Cmd as RunnerCmd, Config as RunnerConfig,
    ExitWith as RunnerExitWith, Fixtures as RunnerFixtures, Function as RunnerFunction,
    LibraryBenchmark as RunnerLibraryBenchmark,
    LibraryBenchmarkConfig as RunnerLibraryBenchmarkConfig,
    LibraryBenchmarkGroup as RunnerLibraryBenchmarkGroup, Options as RunnerOptions,
    RawCallgrindArgs as RunnerRawCallgrindArgs, Run as RunnerRun,
};

#[derive(Debug, Default)]
pub struct Config {
    pub inner: RunnerConfig,
}

impl From<Config> for RunnerConfig {
    fn from(value: Config) -> Self {
        value.inner
    }
}

#[derive(Debug, Default)]
pub struct BinaryBenchmark {
    pub inner: RunnerBinaryBenchmark,
}

impl From<BinaryBenchmark> for RunnerBinaryBenchmark {
    fn from(value: BinaryBenchmark) -> Self {
        value.inner
    }
}

#[derive(Debug, Clone)]
pub struct Cmd {
    pub inner: RunnerCmd,
}

impl Cmd {
    pub fn new<T, U>(orig: T, cmd: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self {
            inner: RunnerCmd {
                display: orig.as_ref().to_owned(),
                cmd: cmd.as_ref().to_owned(),
            },
        }
    }
}

impl From<Cmd> for RunnerCmd {
    fn from(value: Cmd) -> Self {
        value.inner
    }
}
