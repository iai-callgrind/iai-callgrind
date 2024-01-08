mod args;
mod bin_bench;
pub mod callgrind;
mod costs;
pub mod dhat;
mod format;
mod lib_bench;
mod meta;
pub mod summary;
pub mod tool;

use std::io::{stdin, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::debug;

use self::meta::Metadata;
use self::summary::BenchmarkKind;
use crate::api::{BinaryBenchmark, LibraryBenchmark};
use crate::error::Error;

pub mod envs {
    pub const IAI_CALLGRIND_COLOR: &str = "IAI_CALLGRIND_COLOR";
    pub const IAI_CALLGRIND_LOG: &str = "IAI_CALLGRIND_LOG";

    pub const CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
    pub const CARGO_TARGET_DIR: &str = "CARGO_TARGET_DIR";
    pub const CARGO_TERM_COLOR: &str = "CARGO_TERM_COLOR";
}

#[derive(Debug)]
pub struct Config {
    package_dir: PathBuf,
    bench_file: PathBuf,
    module: String,
    bench_bin: PathBuf,
    meta: Metadata,
}

fn compare_versions(runner_version: String, library_version: String) -> Result<()> {
    match version_compare::compare(&runner_version, &library_version) {
        Ok(cmp) => match cmp {
            version_compare::Cmp::Lt | version_compare::Cmp::Gt => {
                return Err(Error::VersionMismatch(cmp, runner_version, library_version).into());
            }
            // version_compare::compare only returns Cmp::Lt, Cmp::Gt and Cmp::Eq so the versions
            // are equal here
            _ => {}
        },
        // iai-callgrind versions before 0.3.0 don't submit the version
        Err(()) => {
            return Err(Error::VersionMismatch(
                version_compare::Cmp::Ne,
                runner_version,
                library_version,
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
    let mut args_iter = std::env::args_os();

    // This unwrap is safe since the first argument is alway the executable
    let runner = PathBuf::from(args_iter.next().unwrap());
    debug!("Runner executable: '{}'", runner.display());

    // The following unwraps are safe because these arguments are assuredly submitted by the
    // iai_callgrind::main macro
    let library_version = args_iter.next().unwrap().to_str().unwrap().to_owned();
    let runner_version = env!("CARGO_PKG_VERSION").to_owned();
    let bench_kind = match args_iter.next().unwrap().to_str().unwrap() {
        "--lib-bench" => BenchmarkKind::LibraryBenchmark,
        "--bin-bench" => BenchmarkKind::BinaryBenchmark,
        kind => panic!("Invalid benchmark kind: '{kind}'"),
    };

    let package_dir = PathBuf::from(args_iter.next().unwrap());
    let bench_file = PathBuf::from(args_iter.next().unwrap());
    let module = args_iter.next().unwrap().to_str().unwrap().to_owned();
    let bench_bin = PathBuf::from(args_iter.next().unwrap());
    let num_bytes = args_iter
        .next()
        .unwrap()
        .to_string_lossy()
        .parse::<usize>()
        .unwrap();

    match bench_kind {
        BenchmarkKind::LibraryBenchmark => {
            let benchmark: LibraryBenchmark = receive_benchmark(num_bytes)?;
            let meta = Metadata::new(&benchmark.command_line_args)?;
            let config = Config {
                package_dir,
                bench_file,
                module,
                bench_bin,
                meta,
            };

            compare_versions(runner_version, library_version)?;
            lib_bench::run(benchmark, config)
        }
        BenchmarkKind::BinaryBenchmark => {
            let benchmark: BinaryBenchmark = receive_benchmark(num_bytes)?;
            let meta = Metadata::new(&benchmark.command_line_args)?;
            let config = Config {
                package_dir,
                bench_file,
                module,
                bench_bin,
                meta,
            };

            compare_versions(runner_version, library_version)?;
            bin_bench::run(benchmark, config)
        }
    }
}
