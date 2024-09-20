use benchmark_tests::{find_primes_multi_thread, thread_in_thread_with_instrumentation};

fn main() {
    let mut args_iter = std::env::args().skip(1);
    match args_iter.next() {
        Some(value) if value.as_str() == "--thread-in-thread" => {
            iai_callgrind::client_requests::callgrind::start_instrumentation();
            let result = thread_in_thread_with_instrumentation();
            iai_callgrind::client_requests::callgrind::stop_instrumentation();
            result
        }
        Some(value) => {
            let num_threads = value.parse::<usize>().unwrap();
            find_primes_multi_thread(num_threads)
        }
        None => find_primes_multi_thread(0),
    };
}
