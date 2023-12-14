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
    ($user_req:path) => {{ $user_req as cty::c_uint > 0x1000 }};
}

cfg_if! {
    if #[cfg(feature = "client_requests")] {
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($arg:tt)*) => {{
                $crate::client_requests::__valgrind_print(format!($($arg)*));
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println {
            () => { valgrind_printf!("\n") };
            ($($arg:tt)*) => {{
                $crate::client_requests::__valgrind_print(format!("{}\n", format_args!($($arg)*)));
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => {{
                $crate::client_requests::__valgrind_print_backtrace(format!($($arg)*));
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace {
            () => { valgrind_printf_backtrace!("\n") };
            ($($arg:tt)*) => {{
                $crate::client_requests::__valgrind_print_backtrace(format!("{}\n", format_args!($($arg)*)));
            }};
        }
    } else {
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($arg:tt)*) => {};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println {
            ($($arg:tt)*) => {};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => {};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace {
            ($($arg:tt)*) => {};
        }
    }
}

mod arch;
mod bindings;
pub mod callgrind;
#[cfg(client_requests_support = "native")]
mod native_bindings;
pub mod valgrind;

use std::ffi::CString;

use arch::imp::valgrind_do_client_request_expr;
use arch::valgrind_do_client_request_stmt;
use cfg_if::cfg_if;

fn fatal_error(func: &str) -> ! {
    panic!(
        "FATAL: {}::{func} not available! You may need update your installed valgrind version. The
        valgrind version of the active valgrind.h header file is {}.{}. Exiting...",
        module_path!(),
        bindings::__VALGRIND_MAJOR__,
        bindings::__VALGRIND_MINOR__
    );
}

#[doc(hidden)]
#[inline(always)]
pub fn __valgrind_print(string: String) {
    let c_string =
        CString::new(string).expect("A valid string should not contain \\0 bytes in the middle");

    valgrind_do_client_request_expr(
        0,
        bindings::IC_ValgrindClientRequest::IC_PRINTF_VALIST_BY_REF as cty::c_uint,
        c_string.as_ptr() as usize,
        0,
        0,
        0,
        0,
    );
}

#[doc(hidden)]
#[inline(always)]
pub fn __valgrind_print_backtrace(string: String) {
    let c_string =
        CString::new(string).expect("A valid string should not contain \\0 bytes in the middle");

    valgrind_do_client_request_expr(
        0,
        bindings::IC_ValgrindClientRequest::IC_PRINTF_BACKTRACE_VALIST_BY_REF as cty::c_uint,
        c_string.as_ptr() as usize,
        0,
        0,
        0,
        0,
    );
}
