use client_request_tests::MARKER;
use iai_callgrind::client_requests::{self};
use iai_callgrind::{cstring, valgrind_println_unchecked};

fn leak_memory() {
    for _ in 0..1 {
        let leaked_box = Box::leak(Box::new(vec![1]));
        unsafe {
            valgrind_println_unchecked!(
                "First value of leaked memory: {}",
                leaked_box.get_unchecked(0)
            )
        };
        let _ = leaked_box;
    }
}

fn main() {
    unsafe { valgrind_println_unchecked!("{MARKER}") };

    unsafe { client_requests::valgrind::clo_change(cstring!("--leak-check=summary\0")) };

    client_requests::memcheck::do_leak_check();
    let _ = client_requests::memcheck::count_leaks();

    leak_memory();

    client_requests::memcheck::do_leak_check();
    let _ = client_requests::memcheck::count_leaks();

    leak_memory();

    client_requests::memcheck::do_new_leak_check();
    let _ = client_requests::memcheck::count_leaks();

    std::process::exit(client_requests::valgrind::running_on_valgrind() as i32);
}
