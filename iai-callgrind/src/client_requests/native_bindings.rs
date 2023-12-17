#![allow(unused)]

extern "C" {
    pub fn valgrind_do_client_request_expr(
        default: usize,
        request: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
    ) -> usize;

    pub fn valgrind_printf(addr: *const cty::c_char) -> usize;
    pub fn valgrind_printf_backtrace(addr: *const cty::c_char) -> usize;
}
