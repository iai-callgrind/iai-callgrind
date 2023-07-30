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

mod macros;

use std::ffi::OsString;
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
#[derive(Debug)]
pub struct Options {
    /// TODO: DOCUMENT
    pub env_clear: bool,
    /// TODO: DOCUMENT
    pub current_dir: Option<PathBuf>,
    /// TODO: DOCUMENT
    pub entry_point: Option<String>,
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
}

/// TODO: DOCUMENT
#[derive(Debug)]
pub struct OptionsParser {
    /// TODO: DOCUMENT
    pub options: Options,
}

impl OptionsParser {
    /// TODO: DOCUMENT
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    /// TODO: DOCUMENT
    pub fn into_arg(self) -> OsString {
        let mut arg = OsString::new();
        if !self.options.env_clear {
            arg.push(format!("'env_clear={}'", self.options.env_clear));
        }
        if let Some(dir) = self.options.current_dir {
            if !arg.is_empty() {
                arg.push(",");
            }
            arg.push("'current_dir=");
            arg.push(dir);
            arg.push("'");
        }
        if let Some(entry_point) = self.options.entry_point {
            if !arg.is_empty() {
                arg.push(",");
            }
            arg.push("'entry_point=");
            arg.push(entry_point);
            arg.push("'");
        }
        arg
    }

    /// TODO: DOCUMENT
    ///
    /// # Panics
    pub fn from_arg(self, arg: &str) -> Option<Options> {
        let mut options = Options::new();
        for opt in arg.strip_prefix('\'')?.strip_suffix('\'')?.split("','") {
            match opt.split_once('=') {
                Some(("env_clear", value)) => options.env_clear = value.parse().unwrap(),
                Some(("current_dir", value)) => options.current_dir = Some(PathBuf::from(value)),
                Some(("entry_point", value)) => options.entry_point = Some(value.to_owned()),
                Some(_) | None => return None,
            }
        }
        Some(options)
    }
}

impl Default for OptionsParser {
    fn default() -> Self {
        Self::new(Options::default())
    }
}
