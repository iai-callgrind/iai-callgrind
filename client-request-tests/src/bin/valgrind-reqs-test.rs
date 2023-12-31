use client_request_tests::MARKER;
use iai_callgrind::{client_requests, valgrind_println_unchecked};

fn main() {
    unsafe { valgrind_println_unchecked!("{MARKER}") };
    std::process::exit(client_requests::valgrind::running_on_valgrind() as i32);
}
