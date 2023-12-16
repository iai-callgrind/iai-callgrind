//! All client requests from the `dhat.h` header file
//!
//! See also the [DHAT documentation](https://valgrind.org/docs/manual/dh-manual.html#dh-manual)

use super::arch::valgrind_do_client_request_stmt;
use super::{bindings, fatal_error};

/// Record an ad hoc event
///
/// If DHAT is invoked with `--mode=ad-hoc`, instead of profiling heap operations (allocations and
/// deallocations), it profiles calls to this `ad_hoc_event` client request.
///
/// The meaning of the `weight` argument will depend on what the event represents, which is up to
/// the user. If no meaningful `weight` argument exists, just use 1.
///
/// See also [Ad hoc profiling](https://valgrind.org/docs/manual/dh-manual.html#dh-manual.ad-hoc-profiling)
#[inline(always)]
pub fn ad_hoc_event(weight: usize) {
    do_client_request!(
        "dhat::ad_hoc_event",
        bindings::IC_DHATClientRequest::IC_DHAT_AD_HOC_EVENT,
        weight,
        0,
        0,
        0,
        0
    );
}

/// For access to count histograms of memory larger than 1k
///
/// The size of the blocks that measure and display access counts is limited to 1024 bytes. This is
/// done to limit the performance overhead and also to keep the size of the generated output
/// reasonable. However, it is possible to override this limit using this client request. The
/// use-case for this is to first run DHAT normally, and then identify any large blocks that you
/// would like to further investigate with access count histograms. The function call should be
/// placed immediately after the call to the allocator, and use the pointer returned by the
/// allocator.
///
/// See also [Access Counts](https://valgrind.org/docs/manual/dh-manual.html#dh-access-counts)
#[inline(always)]
pub fn histogram_memory(addr: *const ()) {
    do_client_request!(
        "dhat::histogram_memory",
        bindings::IC_DHATClientRequest::IC_DHAT_HISTOGRAM_MEMORY,
        addr as usize,
        0,
        0,
        0,
        0
    );
}
