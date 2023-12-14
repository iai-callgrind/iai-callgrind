//! TODO: DOCS
use std::ffi::CString;

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
        let c_string = CString::new(string)
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
