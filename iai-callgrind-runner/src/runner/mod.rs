mod bin_bench;
pub mod callgrind;
mod lib_bench;
mod meta;
mod print;

use std::path::PathBuf;

use log::debug;

pub mod envs {
    pub const IAI_CALLGRIND_ALLOW_ASLR: &str = "IAI_CALLGRIND_ALLOW_ASLR";
    pub const IAI_CALLGRIND_COLOR: &str = "IAI_CALLGRIND_COLOR";
    pub const IAI_CALLGRIND_LOG: &str = "IAI_CALLGRIND_LOG";

    pub const CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
    pub const CARGO_TARGET_DIR: &str = "CARGO_TARGET_DIR";
    pub const CARGO_TERM_COLOR: &str = "CARGO_TERM_COLOR";
}

pub use crate::error::{IaiCallgrindError, Result};
pub use crate::util::{write_all_to_stderr, write_all_to_stdout};

pub fn run() -> Result<()> {
    let mut args_iter = std::env::args_os();

    let runner = PathBuf::from(args_iter.next().unwrap());
    debug!("Runner executable: '{}'", runner.display());

    let library_version = args_iter.next().unwrap().to_str().unwrap().to_owned();
    let runner_version = env!("CARGO_PKG_VERSION").to_owned();

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
