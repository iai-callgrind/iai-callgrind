//! TODO: DOCS
use crate::client_requests::native_bindings;

/// TODO: DOCS
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
    unsafe {
        native_bindings::valgrind_do_client_request_expr(
            default,
            request as usize,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
        )
    }
}
