use std::thread;

use benchmark_tests::find_primes;

fn simple_threaded(num: usize) {
    let mut handles = vec![];
    let mut low = 0;
    for _ in 0..num {
        let handle = thread::spawn(move || find_primes(low, low + 10000));
        handles.push(handle);

        low += 10000;
    }

    let mut primes = vec![];
    for handle in handles {
        let result = handle.join();
        primes.extend(result.unwrap())
    }

    println!(
        "Number of primes found in the range 0 to {low}: {}",
        primes.len()
    );
}

fn thread_in_thread() {
    let low = 0;
    let high = 10000;
    let handle = thread::spawn(move || {
        let handle = thread::spawn(move || find_primes(low, high));
        handle.join().unwrap()
    });
    let primes = handle.join().unwrap();

    println!(
        "thread in thread: Number of primes found in the range {low} to {high}: {}",
        primes.len()
    );
}

fn main() {
    let mut args_iter = std::env::args().skip(1);
    match args_iter.next() {
        Some(value) if value.as_str() == "--thread-in-thread" => thread_in_thread(),
        Some(value) => {
            let num = value.parse::<usize>().unwrap();
            simple_threaded(num);
        }
        None => simple_threaded(0),
    }
}
