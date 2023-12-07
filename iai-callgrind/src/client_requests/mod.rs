//! The public interface to valgrind's client request
//!
//! TODO: MORE DOCUMENTATION

#![allow(clippy::inline_always)]

/// Return true if a client request is defined and available in the used valgrind version
///
/// We do this check to avoid incompatibilities with older valgrinds version which might not have
/// all client requests available, we're offering within this module.
///
/// We're only using constant values known at compile time, which the compiler will finally optimize
/// away, so this macro costs us nothing.
macro_rules! is_def {
    ($user_req:path) => {{ $user_req as cty::c_uint > 0x0000_ffff }};
}

mod arch;
mod bindings;
#[cfg(client_requests_support = "native")]
mod native_bindings;

use arch::imp::valgrind_do_client_request_expr;
use arch::valgrind_do_client_request_stmt;

fn fatal_error(func: &str) -> ! {
    panic!(
        "FATAL: {}::{func} not available! You may need update your installed valgrind version. The
        valgrind version of the active valgrind.h header file is {}.{}. Exiting...",
        module_path!(),
        bindings::__VALGRIND_MAJOR__,
        bindings::__VALGRIND_MINOR__
    );
}

/// TODO: DOCS
pub mod valgrind {
    use super::{
        bindings, fatal_error, valgrind_do_client_request_expr, valgrind_do_client_request_stmt,
    };

    /// TODO: DOCS
    #[inline(always)]
    pub fn running_on_valgrind() -> usize {
        if is_def!(bindings::IC_ValgrindClientRequest::IC_RUNNING_ON_VALGRIND) {
            valgrind_do_client_request_expr(
                0,
                bindings::IC_ValgrindClientRequest::IC_RUNNING_ON_VALGRIND as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            )
        } else {
            fatal_error("valgrind::running_on_valgrind");
        }
    }

    /// TODO: DOCS
    #[inline(always)]
    pub fn discard_translations(addr: *const (), len: usize) {
        if is_def!(bindings::IC_ValgrindClientRequest::IC_VALGRIND_DISCARD_TRANSLATIONS) {
            valgrind_do_client_request_stmt(
                bindings::IC_ValgrindClientRequest::IC_VALGRIND_DISCARD_TRANSLATIONS as cty::c_uint,
                addr as usize,
                len,
                0,
                0,
                0,
            );
        } else {
            fatal_error("valgrind::discard_translations");
        }
    }
}

/// TODO: DOCS
pub mod callgrind {
    use super::{bindings, fatal_error, valgrind_do_client_request_stmt};

    /// TODO: DOCS
    #[inline(always)]
    pub fn dump_stats() {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_DUMP_STATS) {
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_DUMP_STATS as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::dump_stats");
        }
    }

    /// TODO: DOCS
    ///
    /// # Panics
    ///
    /// null bytes
    #[inline(always)]
    pub fn dump_stats_at(string: &str) {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_DUMP_STATS_AT) {
            let c_string = std::ffi::CString::new(string)
                .expect("A valid string should not contain \\0 bytes in the middle");
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_DUMP_STATS_AT as cty::c_uint,
                c_string.as_ptr() as usize,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::dump_stats_at");
        }
    }

    /// .
    #[inline(always)]
    pub fn zero_stats() {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_ZERO_STATS) {
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_ZERO_STATS as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::zero_stats");
        }
    }

    /// .
    #[inline(always)]
    pub fn toggle_collect() {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_TOGGLE_COLLECT) {
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_TOGGLE_COLLECT as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::toggle_collect");
        }
    }

    /// .
    #[inline(always)]
    pub fn start_instrumentation() {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_START_INSTRUMENTATION) {
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_START_INSTRUMENTATION as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::start_instrumentation");
        }
    }

    /// .
    #[inline(always)]
    pub fn stop_instrumentation() {
        if is_def!(bindings::IC_CallgrindClientRequest::IC_STOP_INSTRUMENTATION) {
            valgrind_do_client_request_stmt(
                bindings::IC_CallgrindClientRequest::IC_STOP_INSTRUMENTATION as cty::c_uint,
                0,
                0,
                0,
                0,
                0,
            );
        } else {
            fatal_error("callgrind::stop_instrumentation");
        }
    }
}
