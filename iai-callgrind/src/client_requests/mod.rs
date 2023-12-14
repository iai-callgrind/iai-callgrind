//! The public interface to valgrind's client request
//!
//! You can use these macros to manipulate and query Valgrind's execution inside your own programs.
//!
//! The resulting executables will still run without Valgrind, just a little bit more slowly than
//! they otherwise would, but otherwise unchanged.
//!
//! TODO: Tell about the features and how to use them

#![allow(clippy::inline_always)]

/// Return true if a client request is defined and available in the used valgrind version
///
/// We do this check to avoid incompatibilities with older valgrinds version which might not have
/// all client requests available we're offering.
///
/// We're only using constant values known at compile time, which the compiler will finally optimize
/// away, so this macro costs us nothing.
macro_rules! is_def {
    ($user_req:path) => {{ $user_req as cty::c_uint > 0x1000 }};
}

/// The macro which uses [`valgrind_do_client_request_stmt`] or [`valgrind_do_client_request_expr`]
/// to execute the client request.
///
/// For internal use only!
///
/// This macro has two forms: The first takes 7 arguments `name, request, arg1, ..., arg5` and
/// returns `()`. The expanded macro of this form calls [`valgrind_do_client_request_stmt`]. The
/// second first has 8 arguments `name, default, request, arg1, ..., arg5` returning a `usize`. The
/// expanded macro of this form calls [`valgrind_do_client_request_expr`].
///
/// Both forms will raise a [`fatal_error`] in case the [`is_def`] macro returns false.
macro_rules! do_client_request {
    ($name:literal, $request:path, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {{
        if is_def!($request) {
            valgrind_do_client_request_stmt(
                $request as cty::c_uint,
                $arg1,
                $arg2,
                $arg3,
                $arg4,
                $arg5,
            );
        } else {
            fatal_error($name);
        }
    }};
    (
        $name:literal,
        $default:expr,
        $request:path,
        $arg1:expr,
        $arg2:expr,
        $arg3:expr,
        $arg4:expr,
        $arg5:expr
    ) => {{
        if is_def!($request) {
            valgrind_do_client_request_expr(
                $default,
                $request as cty::c_uint,
                $arg1,
                $arg2,
                $arg3,
                $arg4,
                $arg5,
            )
        } else {
            fatal_error($name);
        }
    }};
}

/// Conveniently create a `\0`-byte terminated [`std::ffi::CString`] from a literal string
///
/// The string literal passed to this macro must not end with a `\0`-byte.
///
/// # Safety
///
/// This macro is unsafe but convenient and very efficient. It is your responsibility to ensure that
/// the input string literal does not contain any `\0` bytes.
#[macro_export]
macro_rules! cstring {
    ($string:literal) => {{ std::ffi::CString::from_vec_with_nul_unchecked(concat!($string, "\0").as_bytes().to_vec()) }};
}

/// Conveniently create a `\0`-byte terminated [`std::ffi::CString`] from a format string
///
/// The format string passed to this macro must not end with a `\0`-byte.
///
/// # Safety
///
/// The same safety conditions as in the [`cstring`] macro apply here
#[macro_export]
macro_rules! format_cstring {
    ($($args:tt)*) => {{ std::ffi::CString::from_vec_with_nul_unchecked(format!("{}\0", format_args!($($args)*)).into_bytes()) }};
}

cfg_if! {
    if #[cfg(feature = "client_requests")] {
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($args:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\0", format_args!($($args)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print(
                                c_string.as_ptr() as *const ()
                            );
                        }
                        Ok(())
                    },
                    Err(error) => Err(
                        $crate::client_requests::error::ClientRequestError::from(error)
                    )
                }
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_unchecked {
            ($($args:tt)*) => {{
                let string = format!("{}\0", format_args!($($args)*));
                $crate::client_requests::__valgrind_print(string.as_ptr() as *const ());
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println {
            () => { $crate::valgrind_printf!("\n") };
            ($($arg:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\n\0", format_args!($($arg)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print(
                                c_string.as_ptr() as *const ()
                            );
                        }
                        Ok(())
                    },
                    Err(error) => Err(
                        $crate::client_requests::error::ClientRequestError::from(error)
                    )
                }
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_unchecked {
            () => { $crate::valgrind_printf_unchecked!("\n") };
            ($($arg:tt)*) => {{
                let string = format!("{}\n\0", format_args!($($arg)*));
                $crate::client_requests::__valgrind_print(string.as_ptr() as *const ());
            }};
        }
        ///
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\0", format_args!($($arg)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print_backtrace(
                                c_string.as_ptr() as *const ()
                            );
                        }
                        Ok(())
                    },
                    Err(error) => Err(
                        $crate::client_requests::error::ClientRequestError::from(error)
                    )
                }
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace_unchecked {
            ($($arg:tt)*) => {{
                let string = format!("{}\0", format_args!($($arg)*));
                $crate::client_requests::__valgrind_print_backtrace(string.as_ptr() as *const ());
            }};
        }
        ///
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace {
            () => { $crate::valgrind_printf_backtrace!("\n") };
            ($($arg:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\n\0", format_args!($($arg)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print_backtrace(
                                c_string.as_ptr() as *const ()
                            );
                        }
                        Ok(())
                    },
                    Err(error) => Err(
                        $crate::client_requests::error::ClientRequestError::from(error)
                    )
                }
            }};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace_unchecked {
            () => { $crate::valgrind_printf_backtrace_unchecked!("\n") };
            ($($arg:tt)*) => {{
                let string = format!("{}\n\0", format_args!($($arg)*));
                unsafe {
                    $crate::client_requests::__valgrind_print_backtrace(
                        string.as_ptr() as *const ()
                    );
                }
            }};
        }
    } else {
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($arg:tt)*) => { Ok(()) };
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_unchecked {
            ($($arg:tt)*) => {};
        }
        ///
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println {
            ($($arg:tt)*) => { Ok(()) };
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_unchecked {
            ($($arg:tt)*) => {};
        }
        ///
        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => { Ok(()) };
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_printf_backtrace_unchecked {
            ($($arg:tt)*) => {};
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace {
            ($($arg:tt)*) => { Ok(()) };
        }

        /// TODO: DOCS
        #[macro_export]
        macro_rules! valgrind_println_backtrace_unchecked {
            ($($arg:tt)*) => {};
        }
    }
}

mod arch;
mod bindings;
pub mod callgrind;
pub mod error;
#[cfg(client_requests_support = "native")]
mod native_bindings;
pub mod valgrind;

use arch::imp::valgrind_do_client_request_expr;
use arch::valgrind_do_client_request_stmt;
use cfg_if::cfg_if;

/// The `ThreadId` is used by some client requests to represent the `tid` which valgrind uses or
/// returns
///
/// This type has no relationship to [`std::thread::ThreadId`]!
pub type ThreadId = usize;

/// The `StackId` is used and returned by some client requests and represents an id on valgrind's
/// stack
pub type StackId = usize;

/// The raw file descriptor number
///
/// This type has no relationship to [`std::os::unix::RawFd`]
pub type RawFd = cty::c_int;

/// Valgrind's version number from the `valgrind.h` file
///
/// Note that the version numbers were introduced at version 3.6 and so would not exist in version
/// 3.5 or earlier. `VALGRIND_VERSION` is None is this case, else it is a tuple
/// `(MAJOR, MINOR)`
pub const VALGRIND_VERSION: Option<(u32, u32)> = {
    if bindings::IC_VALGRIND_MAJOR == 0 {
        None
    } else {
        Some((bindings::IC_VALGRIND_MAJOR, bindings::IC_VALGRIND_MINOR))
    }
};

fn fatal_error(func: &str) -> ! {
    panic!(
        "{0}: FATAL: {0}::{func} not available! You may need update your installed valgrind \
         version. The valgrind version in the valgrind.h header file is {1}. Exiting...",
        module_path!(),
        if let Some((major, minor)) = VALGRIND_VERSION {
            format!("{major}.{minor}")
        } else {
            "< 3.6".to_owned()
        }
    );
}

// #[doc(hidden)]
// #[inline(always)]
// pub fn __valgrind_print<T>(c_string: T)
// where
//     T: AsRef<CStr>,
// {
//     valgrind_do_client_request_expr(
//         0,
//         bindings::IC_ValgrindClientRequest::IC_PRINTF_VALIST_BY_REF as cty::c_uint,
//         c_string.as_ref().as_ptr() as usize,
//         0,
//         0,
//         0,
//         0,
//     );
// }

#[doc(hidden)]
#[inline(always)]
pub unsafe fn __valgrind_print(ptr: *const ()) {
    valgrind_do_client_request_expr(
        0,
        bindings::IC_ValgrindClientRequest::IC_PRINTF_VALIST_BY_REF as cty::c_uint,
        ptr as usize,
        0,
        0,
        0,
        0,
    );
}
//
// #[doc(hidden)]
// #[inline(always)]
// pub fn __valgrind_print_backtrace<T>(c_string: T)
// where
//     T: AsRef<CStr>,
// {
//     valgrind_do_client_request_expr(
//         0,
//         bindings::IC_ValgrindClientRequest::IC_PRINTF_BACKTRACE_VALIST_BY_REF as cty::c_uint,
//         c_string.as_ref().as_ptr() as usize,
//         0,
//         0,
//         0,
//         0,
//     );
// }

#[doc(hidden)]
#[inline(always)]
pub unsafe fn __valgrind_print_backtrace(ptr: *const ()) {
    valgrind_do_client_request_expr(
        0,
        bindings::IC_ValgrindClientRequest::IC_PRINTF_BACKTRACE_VALIST_BY_REF as cty::c_uint,
        ptr as usize,
        0,
        0,
        0,
        0,
    );
}
