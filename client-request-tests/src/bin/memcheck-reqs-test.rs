use client_request_tests::MARKER;
use iai_callgrind::client_requests::memcheck::LeakCounts;
use iai_callgrind::client_requests::{self};
use iai_callgrind::{cstring, valgrind_println_unchecked};

#[track_caller]
fn assert_leaks(
    leaks: LeakCounts,
    leaked: cty::c_ulong,
    dubious: cty::c_ulong,
    reachable: cty::c_ulong,
    suppressed: cty::c_ulong,
) {
    if client_requests::valgrind::running_on_valgrind() == 0 {
        let exp = LeakCounts::default();
        assert_eq!(leaks, exp);
    } else {
        let exp = LeakCounts {
            leaked,
            dubious,
            reachable,
            suppressed,
        };
        assert_eq!(
            leaks.leaked + leaks.dubious,
            exp.leaked + exp.dubious,
            "leaked + dubious"
        );
        assert_eq!(leaks.reachable, exp.reachable, "reachable");
        assert_eq!(leaks.suppressed, exp.suppressed, "suppressed");
    }
}

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
    let initial_leaks = client_requests::memcheck::count_leaks();

    leak_memory();

    client_requests::memcheck::do_leak_check();
    let leaks = client_requests::memcheck::count_leaks();
    assert_leaks(leaks, 4, 0, initial_leaks.reachable, 0);

    leak_memory();

    client_requests::memcheck::do_new_leak_check();
    let leaks = client_requests::memcheck::count_leaks();
    assert_leaks(leaks, 8, 0, initial_leaks.reachable, 0);

    std::process::exit(client_requests::valgrind::running_on_valgrind() as i32);
}
