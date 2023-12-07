// use iai_callgrind::{client_requests as cr, hello};

fn main() {
    // fn my_func() -> u64 {
    //     println!("woops");
    //     2
    // }

    // iai_callgrind::client_requests::print_bindings();
    dbg!(iai_callgrind::client_requests::valgrind::running_on_valgrind());
    // iai_callgrind::client_requests::valgrind::discard_translations(my_func as *const (), 2);

    // iai_callgrind::client_requests::arch::imp::valgrind_do_client_request_expr(default, request,
    // arg1, arg2, arg3, arg4, arg5)

    // TODO: CLEANUP TEST CODE
    // unsafe {
    //     dbg!(iai_callgrind::client_requests::valgrind::running_on_valgrind());
    //     iai_callgrind::client_requests::valgrind::discard_translations(
    //         my_func as *mut std::ffi::c_void,
    //         2,
    //     );
    // }
    // dbg!(iai_callgrind::client_request!(
    //     valgrind::running_on_valgrind,
    // ));
    // iai_callgrind::client_request!(
    //     cr::valgrind::discard_translations,
    //     my_func as *mut std::ffi::c_void,
    //     2
    // );
    // unsafe {
    //     dbg!(iai_callgrind::client_requests::bindings::valgrind::IS_DEF_VALGRIND_DISCARD_TRANSLATIONS);
    // }
    //
    // hello!(cr::bindings::valgrind::IS_DEF_VALGRIND_DISCARD_TRANSLATIONS);
    // for arg in std::env::args() {
    //     println!("{arg}");
    // }
}
