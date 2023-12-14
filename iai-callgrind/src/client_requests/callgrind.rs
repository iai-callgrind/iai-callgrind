//! TODO: DOCS
use std::ffi::CStr;

use super::{bindings, fatal_error, valgrind_do_client_request_stmt};

/// TODO: DOCS
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

/// TODO: DOCS
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

/// .
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

/// .
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

/// .
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

/// .
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
