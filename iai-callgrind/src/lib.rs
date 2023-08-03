//! library of iai-callgrind with helper structs and methods for the main macro

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

pub use bincode;
pub use serde::{Deserialize, Serialize};

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

/// TODO: DOCUMENT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitWith {
    /// TODO: DOCUMENT
    Success,
    /// TODO: DOCUMENT
    Failure,
    /// TODO: DOCUMENT
    Code(i32),
}

/// TODO: DOCUMENT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    /// TODO: DOCUMENT
    pub env_clear: bool,
    /// TODO: DOCUMENT
    pub current_dir: Option<PathBuf>,
    /// TODO: DOCUMENT
    pub entry_point: Option<String>,
    /// TODO: DOCUMENT
    pub exit_with: Option<ExitWith>,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    /// TODO: DOCUMENT
    pub fn new() -> Self {
        Self {
            env_clear: true,
            current_dir: None,
            entry_point: None,
            exit_with: None,
        }
    }

    /// TODO: DOCUMENT
    pub fn env_clear(mut self, value: bool) -> Self {
        self.env_clear = value;
        self
    }

    /// TODO: DOCUMENT
    pub fn current_dir(mut self, value: PathBuf) -> Self {
        self.current_dir = Some(value);
        self
    }

    /// TODO: DOCUMENT
    pub fn entry_point(mut self, value: &str) -> Self {
        self.entry_point = Some(value.to_owned());
        self
    }

    /// TODO: DOCUMENT
    pub fn exit_with(mut self, value: ExitWith) -> Self {
        self.exit_with = Some(value);
        self
    }
}
