//! The iai-callgrind-runner library

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]
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
#![allow(missing_docs)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "api")]
pub mod api;
#[cfg(feature = "runner")]
mod bin_bench;
#[cfg(feature = "runner")]
mod callgrind;
#[cfg(feature = "runner")]
mod error;
#[cfg(feature = "runner")]
mod lib_bench;
#[cfg(feature = "runner")]
mod util;

#[cfg(feature = "runner")]
use std::path::PathBuf;

#[cfg(feature = "runner")]
pub use error::IaiCallgrindError;
#[cfg(feature = "runner")]
use log::debug;
#[cfg(feature = "runner")]
pub use util::{write_all_to_stderr, write_all_to_stdout};

#[cfg(feature = "runner")]
pub fn run() -> Result<(), IaiCallgrindError> {
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
