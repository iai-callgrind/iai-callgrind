use client_request_tests::MARKER;
use iai_callgrind::client_requests::{self};
use iai_callgrind::cstring;

fn do_work(start: i32) -> i32 {
    let mut sum = start;

    for i in 1..10 {
        sum += i;
    }
    sum
}

fn client_requests_1() -> i32 {
    let mut sum = do_work(0);

    client_requests::callgrind::zero_stats();

    sum += do_work(sum);
    client_requests::callgrind::dump_stats();

    sum += do_work(sum);
    client_requests::callgrind::dump_stats_at(unsafe { cstring!("Please dump here") });

    do_work(sum)
}

fn client_requests_2() -> i32 {
    let mut sum = client_requests_1();

    client_requests::callgrind::toggle_collect();

    sum += client_requests_1();
    client_requests::callgrind::toggle_collect();

    sum
}

fn main() {
    unsafe { iai_callgrind::valgrind_println_unchecked!("{MARKER}") };

    client_requests_2();

    client_requests::callgrind::stop_instrumentation();

    client_requests_2();

    client_requests::callgrind::start_instrumentation();

    client_requests_2();

    client_requests::callgrind::stop_instrumentation();

    client_requests_2();

    client_requests::callgrind::start_instrumentation();

    std::process::exit(client_requests::valgrind::running_on_valgrind() as i32);
}
