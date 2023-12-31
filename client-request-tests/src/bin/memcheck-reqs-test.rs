use std::cell::RefCell;
use std::rc::Rc;

use client_request_tests::MARKER;
use iai_callgrind::client_requests::memcheck::LeakCounts;
use iai_callgrind::client_requests::{self};
use iai_callgrind::{cstring, valgrind_println_unchecked};

struct Left(Option<Rc<Right>>);
struct Right(Option<Rc<RefCell<Left>>>);

#[track_caller]
fn assert_leaks(leaks: LeakCounts, leaked: u64, dubious: u64, reachable: u64, suppressed: u64) {
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
        assert_eq!(leaks, exp);
    }
}

fn main() {
    unsafe { valgrind_println_unchecked!("{MARKER}") };

    unsafe { client_requests::valgrind::clo_change(cstring!("--leak-check=summary\0")) };

    client_requests::memcheck::do_leak_check();
    let leaks = client_requests::memcheck::count_leaks();
    assert_leaks(leaks, 0, 0, 85, 0);

    for _ in 0..1000 {
        let left = Rc::new(RefCell::new(Left(None)));
        let right = Rc::new(Right(Some(Rc::clone(&left))));
        left.borrow_mut().0 = Some(Rc::clone(&right));
    }

    client_requests::memcheck::do_leak_check();
    let leaks = client_requests::memcheck::count_leaks();
    assert_leaks(leaks, 55944, 0, 141, 0);

    for _ in 0..10 {
        let left = Rc::new(RefCell::new(Left(None)));
        let right = Rc::new(Right(Some(Rc::clone(&left))));
        left.borrow_mut().0 = Some(Rc::clone(&right));
    }

    client_requests::memcheck::do_new_leak_check();
    let leaks = client_requests::memcheck::count_leaks();
    assert_leaks(leaks, 56504, 0, 141, 0);

    std::process::exit(client_requests::valgrind::running_on_valgrind() as i32);
}
