//! The `iai-callgrind` rustified interface to [Valgrind's Client Request
//! Mechanism](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)
//!
//! You can use these methods to manipulate and query Valgrind's execution inside `iai-callgrind`
//! benchmarks or your own programs.
//!
//! Valgrind has a trapdoor mechanism via which the client program can pass all manner of requests
//! and queries to Valgrind and the current tool. The so-called client requests are provided to
//! allow you to tell Valgrind facts about the behavior of your program, and also to make queries.
//! In particular, your program can tell Valgrind about things that it otherwise would not know,
//! leading to better results.
//!
//! # Building
//!
//! The client requests need to be built with the valgrind header files. Usually, these header files
//! are installed by your distribution's package manager with the valgrind package into a global
//! include path and you don't need to do anything but activating the `client_requests` feature (see
//! below) of the `iai-callgrind` dependency.
//!
//! If you encounter problems because the valgrind header files cannot be found, first ensure you
//! have installed valgrind and your package manager's package includes the header files. If not or
//! you use a custom build of valgrind, you can point the `IAI_CALLGRIND_VALGRIND_INCLUDE`
//! environment variable to the include path where the valgrind headers can be found. The include
//! directive used by `iai-callgrind` is `#include "valgrind/valgrind.h"` and is prefixed with
//! `valgrind`. For example, if the valgrind header files reside in
//! `/home/foo/repo/valgrind/{valgrind.h, callgrind.h, ...}`, then the environment variable has to
//! point to `IAI_CALLGRIND_VALGRIND_INCLUDE=/home/foo/repo` and not
//! `IAI_CALLGRIND_VALGRIND_INCLUDE=/home/foo/repo/valgrind`.
//!
//! Also, worth to consider is that the build of `iai-callgrind` with client requests takes longer
//! than the build without them.
//!
//! # Module Organization
//!
//! The client requests are organized into modules representing the source header file. So, if you
//! search for a client request originating from the `valgrind.h` header file, the client request
//! can be found in the [`crate::client_requests::valgrind`] module. Instead of using macros like in
//! valgrind we're using functions and small letter names, stripping the prefix if it is equal to
//! the enclosing module. For example the client request `RUNNING_ON_VALGRIND` from the `valgrind.h`
//! file equals [`crate::client_requests::valgrind::running_on_valgrind`] and
//! `VALGRIND_COUNT_ERRORS` from the same `valgrind.h` header file equals
//! [`crate::client_requests::valgrind::count_errors`].
//!
//! The only exception to this rule are the [`crate::valgrind_printf`] macro and its descendents
//! like [`crate::valgrind_printf_unchecked`] which can be found in the root of `iai-callgrind`.
//!
//! # Features
//!
//! The client requests are organized into two separate features. The `client_requests_defs` feature
//! enables the definitions but doesn't run any code yet. To actually run the client requests you
//! need to enable the `client_requests` feature. The `client_requests` feature implies
//! `client_requests_defs`. For example, if you need to include the client requests into your
//! production code, you usually don't want them to run if not running under valgrind in the
//! `iai-callgrind` benchmarks. In order to achieve this, the `client_requests_defs` can be safely
//! included in the production code since the compiler will optimize them completely away. So, in
//! your `Cargo.toml` file, you can do
//!
//! ```toml
//! [dependencies]
//! iai-callgrind = { version = "0.9.0", default-features = false, features = [
//!     "client_requests_defs"
//! ]}
//!
//! [dev-dependencies]
//! iai-callgrind = { version = "0.9.0", features = ["client_requests"] }
//! ```
//!
//! If you would only need the client requests in `iai-callgrind` benchmarks, you only need to add
//! `iai-callgrind` with the `client_requests` feature to your `dev-dependencies`.
//!
//! # Performance and implementation details
//!
//! Depending on the target, the client requests are optimized to run with the same overhead like
//! the original valgrind client requests in C code. The optimizations are based on inline assembly
//! with the `asm!` macro and depend on the availability of it on a specific target. Since inline
//! assembly is not stable on all targets which are supported by valgrind, we cannot provide
//! optimized client requests for them. But, you can still use the non-optimized version on all
//! platforms which would be natively supported by valgrind. In the end, all targets which are
//! covered by valgrind are also covered by `iai-callgrind`.
//!
//! The non-optimized version add overhead because we need to wrap the macro from the header file in
//! a function call. This additional function call equals the additional overhead compared to the
//! original valgrind implementation. Although this is usually not much, we try hard to avoid any
//! overhead to not slow down the benchmarks.
//!
//! Here's a short overview on which targets the optimized client requests are available and why
//! not (Valgrind version = `3.22`)
//!
//! | Target                | Optimized | Reason  |
//! | --------------------- | --------- | ------- |
//! | `x86_64/linux`        | yes | -
//! | `x86_64/freebsd`      | yes | -
//! | `x86_64/apple+darwin` | yes | -
//! | `x86_64/windows+gnu`  | yes | -
//! | `x86_64/solaris`      | yes | -
//! | `x86/linux`           | yes | -
//! | `x86/freebsd`         | yes | -
//! | `x86/apple+darwin`    | yes | -
//! | `x86/windows+gnu`     | yes | -
//! | `x86/solaris`         | yes | -
//! | `x86/windows+msvc`    | no  | TBD
//! | `arm/linux`           | yes | -
//! | `aarch64/linux`       | yes | -
//! | `x86_64/windows+msvc` | no  | unsupported by valgrind
//! | `s390x/linux`         | no  | unstable inline assembly
//! | `mips32/linux`        | no  | unstable inline assembly
//! | `mips64/linux`        | no  | unstable inline assembly
//! | `powerpc/linux`       | no  | unstable inline assembly
//! | `powerpc64/linux`     | no  | unstable inline assembly
//! | `powerpc64le/linux`   | no  | unstable inline assembly
//! | `nanomips/linux`      | no  | valgrind only
//!
//! All other targets you don't find in the table above are also not supported by valgrind, yet.
//!
//! Note this table might quickly become outdated with higher versions of valgrind and you should
//! not rely on it to be up-to-date. As indicated above, the bindings are created dynamically in
//! such a way, that always all targets which are covered by valgrind are also covered by
//! `iai-callgrind`. They just might not have been optimized, yet. If you need to know if your
//! target is supported you should consult the `valgrind.h` header file in the [Valgrind
//! Repository](https://sourceware.org/git/?p=valgrind.git) or have a look at the [Valgrind Release
//! Notes](https://valgrind.org/downloads/current.html)
//!
//! # Sources and additional documentation
//!
//! A lot of the library documentation of the client requests within this module and its submodules
//! is taken from the online manual and the valgrind header files. For more details see also [The
//! Client Request
//! mechanism](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)

#![allow(clippy::inline_always)]

/// Return true if a client request is defined and available in the used valgrind version
///
/// For internal use only!
///
/// We do this check to avoid incompatibilities with older valgrinds version which might not have
/// all client requests available we're offering.
///
/// We're only using constant values known at compile time, which the compiler will finally optimize
/// away, so this macro costs us nothing.
macro_rules! is_def {
    ($user_req:path) => {{
        $user_req as cty::c_uint > 0x1000
    }};
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

/// Convenience macro to create a `\0`-byte terminated [`std::ffi::CString`] from a literal string
///
/// The string literal passed to this macro must contain or end with a `\0`-byte. If you need a
/// checked version of [`std::ffi::CString`] you can use [`std::ffi::CString::new`].
///
/// # Safety
///
/// This macro is unsafe but convenient and efficient. It is your responsibility to ensure that the
/// input string literal does not contain any `\0` bytes.
#[macro_export]
macro_rules! cstring {
    ($string:literal) => {{
        std::ffi::CString::from_vec_with_nul_unchecked(concat!($string, "\0").as_bytes().to_vec())
    }};
}

/// Convenience macro to create a `\0`-byte terminated [`std::ffi::CString`] from a format string
///
/// The format string passed to this macro must not contain or end with a `\0`-byte.
///
/// # Safety
///
/// The same safety conditions as to the [`cstring`] macro apply here
#[macro_export]
macro_rules! format_cstring {
    ($($args:tt)*) => {{
        std::ffi::CString::from_vec_with_nul_unchecked(
            format!("{}\0", format_args!($($args)*)).into_bytes()
        )
    }};
}

cfg_if! {
    if #[cfg(feature = "client_requests")] {
        /// Allow prints to valgrind log
        ///
        /// This macro is a safe variant of the `VALGRIND_PRINTF` function, checking for `\0` bytes in the
        /// formatting string. However, if you're sure there are no `\0` bytes present you can
        /// safely use [`crate::valgrind_printf_unchecked`] which performs better compared to this
        /// macro.
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($args:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\0", format_args!($($args)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print(
                                c_string.as_ptr()
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

        /// Allow prints to valgrind log
        ///
        /// Use this macro only if you are sure there are no `\0`-bytes in the formatted string. If
        /// unsure use the safe [`crate::valgrind_printf`] variant.
        ///
        /// This variant performs better than [`crate::valgrind_printf`].
        #[macro_export]
        macro_rules! valgrind_printf_unchecked {
            ($($args:tt)*) => {{
                let string = format!("{}\0", format_args!($($args)*));
                $crate::client_requests::__valgrind_print(
                    string.as_ptr() as *const $crate::cty::c_char
                );
            }};
        }

        /// Allow prints to valgrind log ending with a newline
        ///
        /// See also [`crate::valgrind_printf`]
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
                                c_string.as_ptr()
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

        /// Allow prints to valgrind log ending with a newline
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_println_unchecked {
            () => { $crate::valgrind_printf_unchecked!("\n") };
            ($($args:tt)*) => {{
                let string = format!("{}\n\0", format_args!($($args)*));
                $crate::client_requests::__valgrind_print(
                    string.as_ptr() as *const $crate::cty::c_char
                );
            }};
        }

        /// Allow prints to valgrind log with a backtrace
        ///
        /// See also [`crate::valgrind_printf`]
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => {{
                match std::ffi::CString::from_vec_with_nul(
                    format!("{}\0", format_args!($($arg)*)).into_bytes()
                ) {
                    Ok(c_string) => {
                        unsafe {
                            $crate::client_requests::__valgrind_print_backtrace(
                                c_string.as_ptr()
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

        /// Allow prints to valgrind log with a backtrace
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_printf_backtrace_unchecked {
            ($($arg:tt)*) => {{
                let string = format!("{}\0", format_args!($($arg)*));
                $crate::client_requests::__valgrind_print_backtrace(
                    string.as_ptr() as *const $crate::cty::c_char
                );
            }};
        }

        /// Allow prints to valgrind log with a backtrace ending the formatted string with a newline
        ///
        /// See also [`crate::valgrind_printf`]
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
                                c_string.as_ptr()
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

        /// Allow prints to valgrind log with a backtrace ending the formatted string with a newline
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_println_backtrace_unchecked {
            () => { $crate::valgrind_printf_backtrace_unchecked!("\n") };
            ($($arg:tt)*) => {{
                let string = format!("{}\n\0", format_args!($($arg)*));
                unsafe {
                    $crate::client_requests::__valgrind_print_backtrace(
                        string.as_ptr() as *const $crate::cty::c_char
                    );
                }
            }};
        }
    } else {
        /// Allow prints to valgrind log
        ///
        /// This macro is a safe variant of the `VALGRIND_PRINTF` function, checking for `\0` bytes in the
        /// formatting string. However, if you're sure there are no `\0` bytes present you can
        /// safely use [`crate::valgrind_printf_unchecked`] which performs better compared to this
        /// macro.
        #[macro_export]
        macro_rules! valgrind_printf {
            ($($arg:tt)*) => {{
                let res: Result<(), $crate::client_requests::error::ClientRequestError> = Ok(());
                res
            }};
        }

        /// Allow prints to valgrind log
        ///
        /// Use this macro only if you are sure there are no `\0`-bytes in the formatted string. If
        /// unsure use the safe [`crate::valgrind_printf`] variant.
        ///
        /// This variant performs better than [`crate::valgrind_printf`].
        #[macro_export]
        macro_rules! valgrind_printf_unchecked {
            ($($arg:tt)*) => {{ $crate::client_requests::__no_op() }};
        }

        /// Allow prints to valgrind log ending with a newline
        ///
        /// See also [`crate::valgrind_printf`]
        #[macro_export]
        macro_rules! valgrind_println {
            ($($arg:tt)*) => {{
                let res: Result<(), $crate::client_requests::error::ClientRequestError> = Ok(());
                res
            }};
        }

        /// Allow prints to valgrind log ending with a newline
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_println_unchecked {
            ($($arg:tt)*) => {{ $crate::client_requests::__no_op() }};
        }

        /// Allow prints to valgrind log with a backtrace
        ///
        /// See also [`crate::valgrind_printf`]
        #[macro_export]
        macro_rules! valgrind_printf_backtrace {
            ($($arg:tt)*) => {{
                let res: Result<(), $crate::client_requests::error::ClientRequestError> = Ok(());
                res
            }};
        }

        /// Allow prints to valgrind log with a backtrace
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_printf_backtrace_unchecked {
            ($($arg:tt)*) => {{ $crate::client_requests::__no_op() }};
        }

        /// Allow prints to valgrind log with a backtrace ending the formatted string with a newline
        ///
        /// See also [`crate::valgrind_printf`]
        #[macro_export]
        macro_rules! valgrind_println_backtrace {
            ($($arg:tt)*) => {{
                let res: Result<(), $crate::client_requests::error::ClientRequestError> = Ok(());
                res
            }};
        }

        /// Allow prints to valgrind log with a backtrace ending the formatted string with a newline
        ///
        /// See also [`crate::valgrind_printf_unchecked`]
        #[macro_export]
        macro_rules! valgrind_println_backtrace_unchecked {
            ($($arg:tt)*) => {{ $crate::client_requests::__no_op() }};
        }
    }
}

mod arch;
mod bindings;
pub mod cachegrind;
pub mod callgrind;
pub mod dhat;
pub mod drd;
pub mod error;
pub mod helgrind;
pub mod memcheck;
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
/// This type has no relationship to the standard library type definition of `RawFd` besides they
/// are wrapping the same type on unix systems.
pub type RawFd = cty::c_int;

/// Valgrind's version number from the `valgrind.h` file
///
/// Note that the version numbers were introduced at valgrind version 3.6 and so would not exist in
/// version 3.5 or earlier. `VALGRIND_VERSION` is None is this case, else it is a tuple `(MAJOR,
/// MINOR)`
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
         version or don't use this client request. The valgrind version of the valgrind.h header \
         file is {1}. Aborting...",
        module_path!(),
        if let Some((major, minor)) = VALGRIND_VERSION {
            format!("{major}.{minor}")
        } else {
            "< 3.6".to_owned()
        }
    );
}

#[doc(hidden)]
#[inline(always)]
pub unsafe fn __valgrind_print(ptr: *const cty::c_char) {
    native_bindings::valgrind_printf(ptr);
}

#[doc(hidden)]
#[inline(always)]
pub unsafe fn __valgrind_print_backtrace(ptr: *const cty::c_char) {
    native_bindings::valgrind_printf_backtrace(ptr);
}

#[doc(hidden)]
#[inline(always)]
pub unsafe fn __no_op() {}
