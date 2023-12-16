//! All client requests from the `callgrind.h` header file
//!
//! See also [Callgrind specific client
//! requests](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.clientrequests)
use std::ffi::CStr;

use super::{bindings, fatal_error, valgrind_do_client_request_stmt};

/// Dump current state of cost centers, and zero them afterwards
///
/// Force generation of a profile dump at specified position in code, for the current thread only.
/// Written counters will be reset to zero.
#[inline(always)]
pub fn dump_stats() {
    do_client_request!(
        "callgrind::dump_stats",
        bindings::IC_CallgrindClientRequest::IC_DUMP_STATS,
        0,
        0,
        0,
        0,
        0
    );
}

/// Dump current state of cost centers, and zero them afterwards stating a reason
///
/// Same as [`dump_stats`], but allows to specify a string to be able to distinguish profile dumps.
///
/// The argument is appended to a string stating the reason which triggered the dump. This string is
/// written as a description field into the profile data dump.
#[inline(always)]
pub fn dump_stats_at<T>(c_str: T)
where
    T: AsRef<CStr>,
{
    do_client_request!(
        "callgrind::dump_stats_at",
        bindings::IC_CallgrindClientRequest::IC_DUMP_STATS_AT,
        c_str.as_ref().as_ptr() as usize,
        0,
        0,
        0,
        0
    );
}

/// Zero cost centers
///
/// Reset the profile counters for the current thread to zero.
#[inline(always)]
pub fn zero_stats() {
    do_client_request!(
        "callgrind::zero_stats",
        bindings::IC_CallgrindClientRequest::IC_ZERO_STATS,
        0,
        0,
        0,
        0,
        0
    );
}

/// Toggles collection state.
///
/// The collection state specifies whether the happening of events should be noted or if they are to
/// be ignored. Events are noted by increment of counters in a cost center
///
/// This allows to ignore events with regard to profile counters. See also valgrind command line
/// options `--collect-atstart` and `--toggle-collect`.
#[inline(always)]
pub fn toggle_collect() {
    do_client_request!(
        "callgrind::toggle_collect",
        bindings::IC_CallgrindClientRequest::IC_TOGGLE_COLLECT,
        0,
        0,
        0,
        0,
        0
    );
}

/// Start full callgrind instrumentation if not already switched on
///
/// When cache simulation is done, it will flush the simulated cache; this will lead to an
/// artificial cache warmup phase afterwards with cache misses which would not have happened in
/// reality.
#[inline(always)]
pub fn start_instrumentation() {
    do_client_request!(
        "callgrind::start_instrumentation",
        bindings::IC_CallgrindClientRequest::IC_START_INSTRUMENTATION,
        0,
        0,
        0,
        0,
        0
    );
}

/// Stop full callgrind instrumentation if not already switched off
///
/// This flushes Valgrinds translation cache, and does no additional instrumentation afterwards,
/// which effectively will run at the same speed as the "none" tool (ie. at minimal slowdown). Use
/// this to bypass Callgrind aggregation for uninteresting code parts. To start Callgrind in this
/// mode to ignore the setup phase, use the valgrind command line option `--instr-atstart=no`.
#[inline(always)]
pub fn stop_instrumentation() {
    do_client_request!(
        "callgrind::stop_instrumentation",
        bindings::IC_CallgrindClientRequest::IC_STOP_INSTRUMENTATION,
        0,
        0,
        0,
        0,
        0
    );
}
