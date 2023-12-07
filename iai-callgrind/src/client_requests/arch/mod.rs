//! Provide the platform dependent implementations of the valgrind.h header macros and functions
//!
//! The rust `asm!` macro is not stable yet for all platforms which valgrind supports, so we can't
//! deliver the client requests for all platforms. The platforms which are currently unsupported by
//! rust but by valgrind are:
//!
//! * s390x-unknown-linux
//! * mips32-unknown-linux
//! * mips64-unknown-linux
//! * powerpc-unknown-linux
//! * powerpc64-unknown-linux
//! * powerpc64le-unknown-linux
//! * nanomips-unknown-linux
//!
//! Despite the above platforms, the following platforms are not implemented with `asm!`, yet:
//!
//! * i686-pc-windows-msvc
//! * arm (linux)
//! * aarch64 (linux)
//!
//! We fall back to `native` for all the platforms above.

// The `client_requests_support` cfg is set in the build script
cfg_if::cfg_if! {
    if #[cfg(not(feature = "client_requests"))] {
        pub mod imp {
            #[inline(always)]
            #[allow(clippy::similar_names)]
            pub fn valgrind_do_client_request_expr(
                default: usize,
                request: cty::c_uint,
                arg1: usize,
                arg2: usize,
                arg3: usize,
                arg4: usize,
                arg5: usize,
            ) -> usize {
                default
            }
        }
    } else if #[cfg(client_requests_support = "x86_64")] {
        #[path = "x86_64.rs"]
        pub mod imp;
    } else if #[cfg(client_requests_support = "x86")] {
        #[path = "x86.rs"]
        pub mod imp;
    } else if #[cfg(client_requests_support = "native")] {
        #[path = "native.rs"]
        pub mod imp;
    } else {
        // We're here when `client_requests_support = "no"`
        compile_error!("Client requests are not supported on this platform");
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "client_requests")] {
        #[inline(always)]
        pub fn valgrind_do_client_request_stmt(
            request: cty::c_uint,
            arg1: usize,
            arg2: usize,
            arg3: usize,
            arg4: usize,
            arg5: usize,
        ) {
            imp::valgrind_do_client_request_expr(0, request, arg1, arg2, arg3, arg4, arg5);
        }
    } else {
        #[inline(always)]
        pub fn valgrind_do_client_request_stmt(
            request: cty::c_uint,
            arg1: usize,
            arg2: usize,
            arg3: usize,
            arg4: usize,
            arg5: usize,
        ) {}
    }
}
