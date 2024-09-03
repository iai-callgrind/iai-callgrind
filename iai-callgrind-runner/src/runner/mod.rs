mod args;
mod bin_bench;
pub mod callgrind;
pub mod common;
pub mod costs;
pub mod dhat;
mod format;
mod lib_bench;
pub mod meta;
pub mod summary;
pub mod tool;

use std::env::ArgsOs;
use std::ffi::OsString;
use std::io::{stdin, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use common::{Config, ModulePath};
use log::debug;

use self::meta::Metadata;
use self::summary::BenchmarkKind;
use crate::api::{BinaryBenchmarkGroups, LibraryBenchmarkGroups};
use crate::error::Error;

pub mod envs {
    pub const IAI_CALLGRIND_COLOR: &str = "IAI_CALLGRIND_COLOR";
    pub const IAI_CALLGRIND_LOG: &str = "IAI_CALLGRIND_LOG";

    pub const CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
    pub const CARGO_TARGET_DIR: &str = "CARGO_TARGET_DIR";
    pub const CARGO_TERM_COLOR: &str = "CARGO_TERM_COLOR";
}

pub const DEFAULT_TOGGLE: &str = "*::__iai_callgrind_wrapper_mod::*";

struct RunnerArgs {
    bench_kind: BenchmarkKind,
    package_dir: PathBuf,
    package_name: String,
    bench_file: PathBuf,
    module: String,
    bench_bin: PathBuf,
    num_bytes: usize,
}

struct RunnerArgsIterator(ArgsOs);

impl RunnerArgs {
    fn new() -> Result<Self> {
        let runner_version = env!("CARGO_PKG_VERSION").to_owned();

        let mut args_iter = RunnerArgsIterator::new();

        let runner = args_iter.next_path()?;
        debug!("Runner executable: '{}'", runner.display());

        let library_version = args_iter.next_string()?;

        compare_versions(runner_version, library_version)?;

        let bench_kind = match args_iter.next_string()?.as_str() {
            "--lib-bench" => BenchmarkKind::LibraryBenchmark,
            "--bin-bench" => BenchmarkKind::BinaryBenchmark,
            kind => {
                return Err(Error::InitError(format!("Invalid benchmark kind: {kind}")).into());
            }
        };

        let package_dir = args_iter.next_path()?;
        let package_name = args_iter.next_string()?;
        let bench_file = args_iter.next_path()?;
        let module = args_iter.next_string()?;
        let bench_bin = args_iter.next_path()?;
        let num_bytes = args_iter
            .next_string()?
            .parse::<usize>()
            .map_err(|_| Error::InitError("Failed to parse number of bytes".to_owned()))?;

        Ok(Self {
            bench_kind,
            package_dir,
            package_name,
            bench_file,
            module,
            bench_bin,
            num_bytes,
        })
    }
}

impl RunnerArgsIterator {
    fn new() -> Self {
        Self(std::env::args_os())
    }

    fn next(&mut self) -> Result<OsString> {
        self.0
            .next()
            .ok_or(Error::InitError("Unexpected number of arguments".to_owned()).into())
    }

    fn next_string(&mut self) -> Result<String> {
        self.next()?
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or(Error::InitError("Invalid utf-8 string".to_owned()).into())
    }

    fn next_path(&mut self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.next()?))
    }
}

fn compare_versions<R, L>(runner_version: R, library_version: L) -> Result<()>
where
    R: AsRef<str>,
    L: AsRef<str>,
{
    match version_compare::compare(runner_version.as_ref(), library_version.as_ref()) {
        Ok(cmp) => match cmp {
            version_compare::Cmp::Lt | version_compare::Cmp::Gt => {
                return Err(Error::VersionMismatch(
                    cmp,
                    runner_version.as_ref().to_owned(),
                    library_version.as_ref().to_owned(),
                )
                .into());
            }
            // version_compare::compare only returns Cmp::Lt, Cmp::Gt and Cmp::Eq so the versions
            // are equal here
            _ => {}
        },
        // iai-callgrind versions before 0.3.0 don't submit the version
        Err(()) => {
            return Err(Error::VersionMismatch(
                version_compare::Cmp::Ne,
                runner_version.as_ref().to_owned(),
                library_version.as_ref().to_owned(),
            )
            .into());
        }
    }

    Ok(())
}

/// Method to read, decode and deserialize the data sent by iai-callgrind
///
/// iai-callgrind uses elements from the [`crate::api`], so the runner can understand which elements
/// can be received by this method
pub fn receive_benchmark<T>(num_bytes: usize) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let mut encoded = vec![];
    let mut stdin = stdin();
    stdin
        .read_to_end(&mut encoded)
        .with_context(|| "Failed to read encoded configuration")?;
    assert!(
        encoded.len() == num_bytes,
        "Bytes mismatch when decoding configuration: Expected {num_bytes} bytes but received: {} \
         bytes",
        encoded.len()
    );

    let benchmark: T =
        bincode::deserialize(&encoded).with_context(|| "Failed to decode configuration")?;

    Ok(benchmark)
}

pub fn run() -> Result<()> {
    let RunnerArgs {
        bench_kind,
        package_dir,
        package_name,
        bench_file,
        module,
        bench_bin,
        num_bytes,
    } = RunnerArgs::new()?;

    match bench_kind {
        BenchmarkKind::LibraryBenchmark => {
            let benchmark_groups: LibraryBenchmarkGroups = receive_benchmark(num_bytes)?;
            let meta = Metadata::new(
                &benchmark_groups.command_line_args,
                &package_name,
                &bench_file,
            )?;
            if meta
                .args
                .filter
                .as_ref()
                .map_or(false, |filter| !filter.apply(&meta.bench_name))
            {
                debug!("Benchmark '{}' is filtered out", bench_file.display());
                return Ok(());
            }

            let config = Config {
                package_dir,
                bench_file,
                module_path: ModulePath::new(&module),
                bench_bin,
                meta,
            };

            lib_bench::run(benchmark_groups, config)
        }
        BenchmarkKind::BinaryBenchmark => {
            let benchmark_groups: BinaryBenchmarkGroups = receive_benchmark(num_bytes)?;
            let meta = Metadata::new(
                &benchmark_groups.command_line_args,
                &package_name,
                &bench_file,
            )?;
            if meta
                .args
                .filter
                .as_ref()
                .map_or(false, |filter| !filter.apply(&meta.bench_name))
            {
                debug!("Benchmark '{}' is filtered out", bench_file.display());
                return Ok(());
            }

            let config = Config {
                package_dir,
                bench_file,
                module_path: ModulePath::new(&module),
                bench_bin,
                meta,
            };

            bin_bench::run(benchmark_groups, config)
        }
    }
}
