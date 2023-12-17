//! TODO: DOCS

use core::arch::asm;

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
    let args: [usize; 6] = [request as usize, arg1, arg2, arg3, arg4, arg5];
    let result;
    // SAFETY: TODO:
    unsafe {
        asm! {
            "rol edi,3",
            "rol edi,13",
            "rol edi,29",
            "rol edi,19",
            "xchg ebx, ebx",
            lateout("edx") result,
            in("eax") args.as_ptr(),
            in("edx") default,
        };
    }
    result
}
