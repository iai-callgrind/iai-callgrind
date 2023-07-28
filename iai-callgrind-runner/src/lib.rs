mod bin_bench;
mod callgrind;
mod lib_bench;
mod util;

use std::process::{Command, Output, Stdio};

// TODO: Replace with platform_info or std::env::consts::ARCH??
fn get_arch() -> String {
    let output = Command::new("uname")
        .arg("-m")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run `uname` to determine machine architecture.");

    String::from_utf8(output.stdout)
        .expect("`uname -m` returned invalid unicode.")
        .trim()
        .to_owned()
}

pub enum IaiCallgrindError {
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(std::io::Error),
    CallgrindLaunchError(Output),
    BenchmarkLaunchError(Output),
}

pub fn run() -> Result<(), IaiCallgrindError> {
    let mut args_iter = std::env::args_os().skip(1);

    let library_version = args_iter.next().unwrap().to_str().unwrap().to_owned();
    let runner_version = env!("CARGO_PKG_VERSION").to_string();

    match version_compare::compare(&runner_version, &library_version) {
        Ok(cmp) => match cmp {
            version_compare::Cmp::Lt | version_compare::Cmp::Gt => {
                return Err(IaiCallgrindError::VersionMismatch(
                    cmp,
                    runner_version,
                    library_version,
                ));
            }
            // version_compare::compare only returns Cmp::Lt, Cmp::Gt and Cmp::Eq so the versions
            // are equal here
            _ => {}
        },
        // iai-callgrind versions before 0.3.0 don't submit the version
        Err(_) => {
            return Err(IaiCallgrindError::VersionMismatch(
                version_compare::Cmp::Ne,
                runner_version,
                library_version,
            ));
        }
    }

    if args_iter.next().unwrap() == "--lib-bench" {
        lib_bench::run(args_iter)
    // it has to be --bin-bench
    } else {
        bin_bench::run(args_iter)
    }
}
