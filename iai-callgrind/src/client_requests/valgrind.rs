//! The client requests from `valgrind.h`
//!
//! TODO: MORE DOCS

use std::ffi::CStr;
use std::usize;

use super::{
    bindings, fatal_error, valgrind_do_client_request_expr, valgrind_do_client_request_stmt, RawFd,
    StackId, ThreadId,
};

/// TODO: DOCS
#[inline(always)]
pub fn running_on_valgrind() -> usize {
    do_client_request!(
        "valgrind::running_on_valgrind",
        0,
        bindings::IC_ValgrindClientRequest::IC_RUNNING_ON_VALGRIND,
        0,
        0,
        0,
        0,
        0
    )
}

/// TODO: DOCS
#[inline(always)]
pub fn discard_translations(addr: *const (), len: usize) {
    do_client_request!(
        "valgrind::discard_translations",
        bindings::IC_ValgrindClientRequest::IC_DISCARD_TRANSLATIONS,
        addr as usize,
        len,
        0,
        0,
        0
    );
}

/// TODO: DOCS
#[inline(always)]
pub fn inner_threads(addr: *const ()) {
    do_client_request!(
        "valgrind::inner_threads",
        bindings::IC_ValgrindClientRequest::IC_INNER_THREADS,
        addr as usize,
        0,
        0,
        0,
        0
    );
}

/// Allow control to move from the simulated CPU to the real CPU, calling an arbitrary function.
///
/// Note that the current [`ThreadId`] is inserted as the first argument.
/// So this call:
///
/// `non_simd_call0(func)`
///
/// requires f to have this signature:
///
/// `usize func(ThreadId tid)`
///
/// Note that these client requests are not entirely reliable. For example, if you call a function
/// with them that subsequently calls printf(), there's a high chance Valgrind will crash.
/// Generally, your prospects of these working are made higher if the called function does not refer
/// to any global variables, and does not refer to other functions (print! et al).
#[allow(clippy::fn_to_numeric_cast_any)]
#[inline(always)]
pub fn non_simd_call0(func: fn(ThreadId) -> usize) -> usize {
    do_client_request!(
        "valgrind::non_simd_call0",
        0,
        bindings::IC_ValgrindClientRequest::IC_CLIENT_CALL0,
        func as *const () as usize,
        0,
        0,
        0,
        0
    )
}

/// Allow control to move from the simulated CPU to the real CPU, calling an arbitrary function.
///
/// See also [`non_simd_call0`]
///
/// # Examples
///
/// ```
/// let num = 42i32;
/// let res = iai_callgrind::client_requests::valgrind::non_simd_call1(
///     |_tid, a| unsafe { ((a as *const i32).as_ref().unwrap() + 2) as usize },
///     (&num) as *const i32 as usize,
/// );
/// ```
#[allow(clippy::fn_to_numeric_cast_any)]
#[inline(always)]
pub fn non_simd_call1(func: fn(ThreadId, usize) -> usize, arg1: usize) -> usize {
    do_client_request!(
        "valgrind::non_simd_call1",
        0,
        bindings::IC_ValgrindClientRequest::IC_CLIENT_CALL1,
        func as *const () as usize,
        arg1,
        0,
        0,
        0
    )
}

/// Allow control to move from the simulated CPU to the real CPU, calling an arbitrary function.
///
/// See also [`non_simd_call0`] and [`non_simd_call1`]
#[allow(clippy::fn_to_numeric_cast_any)]
#[inline(always)]
pub fn non_simd_call2(
    func: fn(ThreadId, usize, usize) -> usize,
    arg1: usize,
    arg2: usize,
) -> usize {
    do_client_request!(
        "valgrind::non_simd_call2",
        0,
        bindings::IC_ValgrindClientRequest::IC_CLIENT_CALL2,
        func as *const () as usize,
        arg1,
        arg2,
        0,
        0
    )
}

/// Allow control to move from the simulated CPU to the real CPU, calling an arbitrary function.
///
/// See also [`non_simd_call0`] and [`non_simd_call1`]
#[allow(clippy::fn_to_numeric_cast_any)]
#[inline(always)]
pub fn non_simd_call3(
    func: fn(ThreadId, usize, usize, usize) -> usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> usize {
    do_client_request!(
        "valgrind::non_simd_call3",
        0,
        bindings::IC_ValgrindClientRequest::IC_CLIENT_CALL3,
        func as *const () as usize,
        arg1,
        arg2,
        arg3,
        0
    )
}

/// Counts the number of errors that have been recorded by a tool.
///
/// The tool must record the errors with `VG_(maybe_record_error)()` or `VG_(unique_error)()`
/// for them to be counted.
#[inline(always)]
pub fn count_errors() -> usize {
    do_client_request!(
        "valgrind::count_errors",
        0,
        bindings::IC_ValgrindClientRequest::IC_COUNT_ERRORS,
        0,
        0,
        0,
        0,
        0
    )
}

/// TODO: DOCS
///
/// # Examples
///
/// ```rust, no_run
/// let vec = Vec::<u64>::with_capacity(10);
/// iai_callgrind::client_requests::valgrind::malloclike_block(
///     vec.as_ptr() as *const (),
///     10 * core::mem::size_of::<u64>(),
///     core::mem::size_of::<u64>(),
///     true,
/// );
/// ```
#[inline(always)]
pub fn malloclike_block(addr: *const (), size: usize, redzone: usize, is_zeroed: bool) {
    do_client_request!(
        "valgrind::malloclike_block",
        bindings::IC_ValgrindClientRequest::IC_MALLOCLIKE_BLOCK,
        addr as usize,
        size,
        redzone,
        usize::from(is_zeroed),
        0
    );
}

/// TODO: DOCS
#[inline(always)]
pub fn resizeinplace_block(addr: *const (), old_size: usize, new_size: usize, redzone: usize) {
    do_client_request!(
        "valgrind::resizeinplace_block",
        bindings::IC_ValgrindClientRequest::IC_RESIZEINPLACE_BLOCK,
        addr as usize,
        old_size,
        new_size,
        redzone,
        0
    );
}

/// TODO: DOCS
#[inline(always)]
pub fn freelike_block(addr: *const (), redzone: usize) {
    do_client_request!(
        "valgrind::freelike_block",
        bindings::IC_ValgrindClientRequest::IC_FREELIKE_BLOCK,
        addr as usize,
        redzone,
        0,
        0,
        0
    );
}

/// TODO: DOCS
#[inline(always)]
pub fn create_mempool(pool: *const (), redzone: usize, is_zeroed: bool) {
    do_client_request!(
        "valgrind::create_mempool",
        bindings::IC_ValgrindClientRequest::IC_CREATE_MEMPOOL,
        pool as usize,
        redzone,
        usize::from(is_zeroed),
        0,
        0
    );
}

/// TODO: DOCS
#[allow(non_snake_case)]
pub mod MempoolFlags {
    /// TODO: DOCS
    pub const DEFAULT: u8 = 0;
    /// TODO: DOCS
    pub const AUTOFREE: u8 = 1;
    /// TODO: DOCS
    pub const METAPOOL: u8 = 2;
}

/// TODO: DOCS
#[inline(always)]
pub fn create_mempool_ext(pool: *const (), redzone: usize, is_zeroed: bool, flags: u8) {
    do_client_request!(
        "valgrind::create_mempool_ext",
        bindings::IC_ValgrindClientRequest::IC_CREATE_MEMPOOL,
        pool as usize,
        redzone,
        usize::from(is_zeroed),
        flags as usize,
        0
    );
}

/// Destroy a memory pool
#[inline(always)]
pub fn destroy_mempool(pool: *const ()) {
    do_client_request!(
        "valgrind::destroy_mempool",
        bindings::IC_ValgrindClientRequest::IC_DESTROY_MEMPOOL,
        pool as usize,
        0,
        0,
        0,
        0
    );
}

/// Associate a piece of memory with a memory pool
#[inline(always)]
pub fn mempool_alloc(pool: *const (), addr: *const (), size: usize) {
    do_client_request!(
        "valgrind::mempool_alloc",
        bindings::IC_ValgrindClientRequest::IC_MEMPOOL_ALLOC,
        pool as usize,
        addr as usize,
        size,
        0,
        0
    );
}

/// Disassociate a piece of memory from a memory pool
#[inline(always)]
pub fn mempool_free(pool: *const (), addr: *const ()) {
    do_client_request!(
        "valgrind::mempool_free",
        bindings::IC_ValgrindClientRequest::IC_MEMPOOL_FREE,
        pool as usize,
        addr as usize,
        0,
        0,
        0
    );
}

/// Disassociate any pieces outside a particular range
#[inline(always)]
pub fn mempool_trim(pool: *const (), addr: *const (), size: usize) {
    do_client_request!(
        "valgrind::mempool_trim",
        bindings::IC_ValgrindClientRequest::IC_MEMPOOL_TRIM,
        pool as usize,
        addr as usize,
        size,
        0,
        0
    );
}

/// Resize and/or move a piece associated with a memory pool
#[inline(always)]
pub fn move_mempool(pool_a: *const (), pool_b: *const ()) {
    do_client_request!(
        "valgrind::move_mempool",
        bindings::IC_ValgrindClientRequest::IC_MOVE_MEMPOOL,
        pool_a as usize,
        pool_b as usize,
        0,
        0,
        0
    );
}

/// Resize and/or move a piece associated with a memory pool
#[inline(always)]
pub fn mempool_change(pool: *const (), addr_a: *const (), addr_b: *const (), size: usize) {
    do_client_request!(
        "valgrind::mempool_change",
        bindings::IC_ValgrindClientRequest::IC_MEMPOOL_CHANGE,
        pool as usize,
        addr_a as usize,
        addr_b as usize,
        size,
        0
    );
}

/// Return true if a mempool exists, else false
#[inline(always)]
pub fn mempool_exists(pool: *const ()) -> bool {
    do_client_request!(
        "valgrind::mempool_exists",
        0,
        bindings::IC_ValgrindClientRequest::IC_MEMPOOL_EXISTS,
        pool as usize,
        0,
        0,
        0,
        0
    ) != 0
}

/// Mark a piece of memory as being a stack. Returns a [`super::StackId`]
///
/// `start` is the lowest addressable stack byte, `end` is the highest addressable stack byte.
#[inline(always)]
pub fn stack_register(start: usize, end: usize) -> StackId {
    do_client_request!(
        "valgrind::stack_register",
        0,
        bindings::IC_ValgrindClientRequest::IC_STACK_REGISTER,
        start,
        end,
        0,
        0,
        0
    )
}

/// Unmark the piece of memory associated with a [`StackId`] as being a stack
#[inline(always)]
pub fn stack_deregister(stack_id: StackId) {
    do_client_request!(
        "valgrind::stack_deregister",
        bindings::IC_ValgrindClientRequest::IC_STACK_DEREGISTER,
        stack_id,
        0,
        0,
        0,
        0
    );
}

/// Change the `start` and `end` address of the [`StackId`]
///
/// `start` is the new lowest addressable stack byte, `end` is the new highest addressable stack
/// byte.
#[inline(always)]
pub fn stack_change(stack_id: StackId, start: usize, end: usize) {
    do_client_request!(
        "valgrind::stack_change",
        bindings::IC_ValgrindClientRequest::IC_STACK_CHANGE,
        stack_id,
        start,
        end,
        0,
        0
    );
}

/// Load PDB debug info for `Wine PE image_map`
///
/// # Panics
///
/// When the raw file descriptor `fd` is smaller than 0
#[inline(always)]
pub fn load_pdb_debuginfo(fd: RawFd, ptr: *const (), total_size: usize, delta: usize) {
    do_client_request!(
        "valgrind::load_pdb_debuginfo",
        bindings::IC_ValgrindClientRequest::IC_LOAD_PDB_DEBUGINFO,
        fd.try_into().expect("A file descriptor should be >= 0"),
        ptr as usize,
        total_size,
        delta,
        0
    );
}

/// Map a code address to a source file name and line number
///
/// `buf64` must point to a 64-byte buffer in the caller's address space. The result will be dumped
/// in there and is guaranteed to be zero terminated. If no info is found, the first byte is set to
/// zero.
#[inline(always)]
pub fn map_ip_to_srcloc(addr: *const (), buf64: *const ()) -> usize {
    do_client_request!(
        "valgrind::map_ip_to_srcloc",
        0,
        bindings::IC_ValgrindClientRequest::IC_MAP_IP_TO_SRCLOC,
        addr as usize,
        buf64 as usize,
        0,
        0,
        0
    )
}

/// Disable error reporting for this thread.
///
/// Behaves in a stack like way, so you can safely call this multiple times provided that
/// [`enable_error_reporting`] is called the same number of times to re-enable reporting. The
/// first call of this macro disables reporting. Subsequent calls have no effect except to increase
/// the number of [`enable_error_reporting`] calls needed to re-enable reporting. Child
/// threads do not inherit this setting from their parents -- they are always created with reporting
/// enabled.
#[inline(always)]
pub fn disable_error_reporting() {
    if is_def!(bindings::IC_ValgrindClientRequest::IC_CHANGE_ERR_DISABLEMENT) {
        valgrind_do_client_request_stmt(
            bindings::IC_ValgrindClientRequest::IC_CHANGE_ERR_DISABLEMENT as cty::c_uint,
            1,
            0,
            0,
            0,
            0,
        );
    } else {
        fatal_error("valgrind::disable_error_reporting");
    }
}

/// Re-enable error reporting
///
/// See also [`disable_error_reporting`]
#[inline(always)]
pub fn enable_error_reporting() {
    do_client_request!(
        "valgrind::enable_error_reporting",
        bindings::IC_ValgrindClientRequest::IC_CHANGE_ERR_DISABLEMENT,
        usize::MAX, // The original code in `valgrind.h` used `-1` as value
        0,
        0,
        0,
        0
    );
}

/// Execute a monitor command from the client program
///
/// If a connection is opened with GDB, the output will be sent according to the output mode set for
/// vgdb. If no connection is opened, output will go to the log output. Returns `false` if command
/// not recognized, `true` otherwise. Note the return value deviates from the original in
/// `valgrind.h` which returns 1 if the command was not recognized and 0 otherwise.
#[inline(always)]
pub fn monitor_command<T>(command: T) -> bool
where
    T: AsRef<CStr>,
{
    do_client_request!(
        "valgrind::monitor_command",
        0,
        bindings::IC_ValgrindClientRequest::IC_GDB_MONITOR_COMMAND,
        command.as_ref().as_ptr() as usize,
        0,
        0,
        0,
        0
    ) != 1
}

/// Change the value of a dynamic command line option
///
/// Note that unknown or not dynamically changeable options will cause a warning message to be
/// output.
#[inline(always)]
pub fn clo_change<T>(option: T)
where
    T: AsRef<CStr>,
{
    do_client_request!(
        "valgrind::clo_change",
        bindings::IC_ValgrindClientRequest::IC_CLO_CHANGE,
        option.as_ref().as_ptr() as usize,
        0,
        0,
        0,
        0
    );
}
