// Copyright (C) 2006-2020 Bart Van Assche <bvanassche@acm.org>.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//
// 2. The origin of this software must not be misrepresented; you must
// not claim that you wrote the original software.  If you use this
// software in a product, an acknowledgment in the product
// documentation would be appreciated but is not required.
//
// 3. Altered source versions must be plainly marked as such, and must
// not be misrepresented as being the original software.
//
// 4. The name of the author may not be used to endorse or promote
// products derived from this software without specific prior written
// permission.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
// OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
// GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// ----------------------------------------------------------------
//
// We're using a lot of the original documentation from the `drd.h` header file with some
// small adjustments, so above is the original license from `drd.h` file.
//
// This file is distributed under the same License as the rest of `iai-callgrind`.
//
// ----------------------------------------------------------------
//
//! All public client requests from the `drd.h` header file
//!
//! The client requests which do nothing or are not implemented (as of valgrind version 3.22) are
//! not available.
//!
//! See also [DRD Client
//! Requests](https://valgrind.org/docs/manual/drd-manual.html#drd-manual.clientreqs)
use std::ffi::CStr;

use super::arch::imp::valgrind_do_client_request_expr;
use super::arch::valgrind_do_client_request_stmt;
use super::{bindings, fatal_error, helgrind};

/// Obtain the thread ID assigned by Valgrind's core
///
/// Valgrind's thread ID's start at one and are recycled in case a thread stops.
#[inline(always)]
pub fn get_valgrind_threadid() -> usize {
    do_client_request!(
        "drd::get_valgrind_threadid",
        0,
        bindings::IC_DRDClientRequest::IC_DRD_GET_VALGRIND_THREAD_ID,
        0,
        0,
        0,
        0,
        0
    )
}

/// Obtain the thread ID assigned by DRD
///
/// These are the thread ID's reported by DRD in data race reports and in trace messages. DRD's
/// thread ID's start at one and are never recycled.
#[inline(always)]
pub fn get_drd_threadid() -> usize {
    do_client_request!(
        "drd::get_drd_threadid",
        0,
        bindings::IC_DRDClientRequest::IC_DRD_GET_DRD_THREAD_ID,
        0,
        0,
        0,
        0,
        0
    )
}

/// Tell DRD not to complain about data races for the specified variable
///
/// Some applications contain intentional races. There exist e.g. applications where the same value
/// is assigned to a shared variable from two different threads. It may be more convenient to
/// suppress such races than to solve these. This client request allows one to suppress such races.
#[inline(always)]
pub fn ignore_var<T>(var: &T) {
    do_client_request!(
        "drd::ignore_var",
        bindings::IC_DRDClientRequest::IC_DRD_START_SUPPRESSION,
        var as *const T as usize,
        core::mem::size_of::<T>(),
        0,
        0,
        0
    );
}

/// Tell DRD to no longer ignore data races for the specified variable that was suppressed via
/// [`ignore_var`]
#[inline(always)]
pub fn stop_ignoring_var<T>(var: &T) {
    do_client_request!(
        "drd::stop_ignoring_var",
        bindings::IC_DRDClientRequest::IC_DRD_FINISH_SUPPRESSION,
        var as *const T as usize,
        core::mem::size_of::<T>(),
        0,
        0,
        0
    );
}

/// Tell DRD to trace all memory accesses for the specified variable until the memory that was
/// allocated for the variable is freed.
///
/// When DRD reports a data race on a specified variable, and it's not immediately clear which
/// source code statements triggered the conflicting accesses, it can be very helpful to trace all
/// activity on the offending memory location.
#[inline(always)]
pub fn trace_var<T>(var: &T) {
    do_client_request!(
        "drd::trace_var",
        bindings::IC_DRDClientRequest::IC_DRD_START_TRACE_ADDR,
        var as *const T as usize,
        core::mem::size_of::<T>(),
        0,
        0,
        0
    );
}

/// Tell DRD to stop tracing memory accesses for the specified variable
#[inline(always)]
pub fn stop_tracing_var<T>(var: &T) {
    do_client_request!(
        "drd::stop_tracing_var",
        bindings::IC_DRDClientRequest::IC_DRD_STOP_TRACE_ADDR,
        var as *const T as usize,
        core::mem::size_of::<T>(),
        0,
        0,
        0
    );
}

/// Create completely arbitrary happens-before edges between threads
///
/// See [`super::helgrind::annotate_happens_before`]
#[inline(always)]
pub fn annotate_happens_before(obj: *const ()) {
    helgrind::annotate_happens_before(obj);
}

/// See [`super::helgrind::annotate_happens_before`]
#[inline(always)]
pub fn annotate_happens_after(obj: *const ()) {
    helgrind::annotate_happens_after(obj);
}

/// Report that a lock has just been created at address `lock`
///
/// See [`super::helgrind::annotate_rwlock_create`]
#[inline(always)]
pub fn annotate_rwlock_create(lock: *const ()) {
    helgrind::annotate_rwlock_create(lock);
}

/// Report that the lock at address `lock` is about to be destroyed
///
/// See [`super::helgrind::annotate_rwlock_create`]
#[inline(always)]
pub fn annotate_rwlock_destroy(lock: *const ()) {
    helgrind::annotate_rwlock_destroy(lock);
}

/// Report that the lock at address `lock` has just been acquired
///
/// See also [`super::helgrind::annotate_rwlock_create`]
#[inline(always)]
pub fn annotate_rwlock_acquired(lock: *const (), is_writer_lock: bool) {
    helgrind::annotate_rwlock_acquired(lock, is_writer_lock);
}

/// Report that the lock at address `lock` is about to be released
///
/// See also [`super::helgrind::annotate_rwlock_create`]
#[inline(always)]
pub fn annotate_rwlock_released(lock: *const (), is_writer_lock: bool) {
    helgrind::annotate_rwlock_released(lock, is_writer_lock);
}

/// Tell DRD that data races at the specified address are expected and must not be reported
///
/// Any races detected on the specified variable are benign and hence should not be reported.
#[inline(always)]
pub fn annotate_benign_race<T>(addr: &T) {
    do_client_request!(
        "drd::annotate_benign_race",
        bindings::IC_DRDClientRequest::IC_DRD_START_SUPPRESSION,
        addr as *const T as usize,
        core::mem::size_of::<T>(),
        0,
        0,
        0
    );
}

/// Same as [`annotate_benign_race`], but applies to the memory range [addr, addr + size).
///
/// Any races detected on the specified variable are benign and hence should not be reported.
#[inline(always)]
pub fn annotate_benign_race_sized<T>(addr: &T, size: usize) {
    do_client_request!(
        "drd::annotate_benign_race_sized",
        bindings::IC_DRDClientRequest::IC_DRD_START_SUPPRESSION,
        addr as *const T as usize,
        size,
        0,
        0,
        0
    );
}

/// Tell DRD to ignore all reads performed by the current thread
#[inline(always)]
pub fn annotate_ignore_reads_begin() {
    do_client_request!(
        "drd::annotate_ignore_reads_begin",
        bindings::IC_DRDClientRequest::IC_DRD_RECORD_LOADS,
        0,
        0,
        0,
        0,
        0
    );
}

/// Tell DRD to no longer ignore the reads performed by the current thread.
#[inline(always)]
pub fn annotate_ignore_reads_end() {
    do_client_request!(
        "drd::annotate_ignore_reads_end",
        bindings::IC_DRDClientRequest::IC_DRD_RECORD_LOADS,
        1,
        0,
        0,
        0,
        0
    );
}

/// Tell DRD to ignore all writes performed by the current thread.
#[inline(always)]
pub fn annotate_ignore_writes_begin() {
    do_client_request!(
        "drd::annotate_ignore_writes_begin",
        bindings::IC_DRDClientRequest::IC_DRD_RECORD_STORES,
        0,
        0,
        0,
        0,
        0
    );
}

/// Tell DRD to no longer ignore the writes performed by the current thread.
#[inline(always)]
pub fn annotate_ignore_writes_end() {
    do_client_request!(
        "drd::annotate_ignore_writes_end",
        bindings::IC_DRDClientRequest::IC_DRD_RECORD_STORES,
        1,
        0,
        0,
        0,
        0
    );
}

/// Tell DRD to ignore all memory accesses performed by the current thread.
#[inline(always)]
pub fn annotate_ignore_reads_and_writes_begin() {
    annotate_ignore_reads_begin();
    annotate_ignore_writes_begin();
}

/// Tell DRD to no longer ignore the memory accesses performed by the current thread.
#[inline(always)]
pub fn annotate_ignore_reads_and_writes_end() {
    annotate_ignore_reads_end();
    annotate_ignore_writes_end();
}

/// Tell DRD that size bytes starting at addr has been allocated by a custom memory allocator
#[inline(always)]
pub fn annotate_new_memory(addr: *const (), size: usize) {
    do_client_request!(
        "drd::annotate_new_memory",
        bindings::IC_DRDClientRequest::IC_DRD_CLEAN_MEMORY,
        addr as usize,
        size,
        0,
        0,
        0
    );
}

/// Ask DRD to report every access to the specified address
///
/// Trace all load and store activity that touches at least the single byte at the address `addr`.
#[inline(always)]
pub fn annotate_trace_memory(addr: *const ()) {
    do_client_request!(
        "drd::annotate_trace_memory",
        bindings::IC_DRDClientRequest::IC_DRD_START_TRACE_ADDR,
        addr as usize,
        core::mem::size_of::<cty::c_char>(),
        0,
        0,
        0
    );
}

/// Tell DRD to assign the specified name to the current thread.
///
/// This name will be used in error messages printed by DRD.
#[inline(always)]
pub fn annotate_thread_name<T>(name: T)
where
    T: AsRef<CStr>,
{
    do_client_request!(
        "drd::annotate_thread_name",
        bindings::IC_DRDClientRequest::IC_DRD_SET_THREAD_NAME,
        name.as_ref().as_ptr() as usize,
        0,
        0,
        0,
        0
    );
}
