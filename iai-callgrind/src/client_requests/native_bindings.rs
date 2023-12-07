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
}
