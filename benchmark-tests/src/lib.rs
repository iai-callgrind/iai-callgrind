pub mod assert;
pub mod common;
pub mod serde;

use std::ffi::OsStr;
use std::io;
use std::process::Output;

pub fn is_prime(num: u64) -> bool {
    if num <= 1 {
        return false;
    }

    for i in 2..=(num as f64).sqrt() as u64 {
        if num % i == 0 {
            return false;
        }
    }

    true
}

#[inline(never)]
pub fn find_primes(low: u64, high: u64) -> Vec<u64> {
    (low..=high).filter(|n| is_prime(*n)).collect()
}

pub fn find_primes_multi_thread(num_threads: usize) -> Vec<u64> {
    let mut handles = vec![];
    let mut low = 0;
    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || find_primes(low, low + 10000));
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

    primes
}

pub fn find_primes_multi_thread_with_instrumentation(num_threads: usize) -> Vec<u64> {
    let mut handles = vec![];
    let mut low = 0;
    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || {
            iai_callgrind::client_requests::callgrind::start_instrumentation();
            iai_callgrind::client_requests::callgrind::toggle_collect();
            let result = find_primes(low, low + 10000);
            iai_callgrind::client_requests::callgrind::toggle_collect();
            iai_callgrind::client_requests::callgrind::stop_instrumentation();
            result
        });
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

    primes
}

pub fn thread_in_thread_with_instrumentation() -> Vec<u64> {
    let low = 0;
    let high = 10000;
    let handle = std::thread::spawn(move || {
        iai_callgrind::client_requests::callgrind::start_instrumentation();
        let handle = std::thread::spawn(move || find_primes(low, high));
        let joined = handle.join().unwrap();
        iai_callgrind::client_requests::callgrind::stop_instrumentation();
        joined
    });
    let primes = handle.join().unwrap();

    println!(
        "thread in thread: Number of primes found in the range {low} to {high}: {}",
        primes.len()
    );

    primes
}

// This function is used to create the worst case array we want to sort with our implementation of
// bubble sort
pub fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

// This function is used to create the best case array we want to sort with our implementation of
// bubble sort
pub fn setup_best_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).collect()
    } else {
        (0..start).collect()
    }
}

pub fn bubble_sort(mut array: Vec<i32>) -> Vec<i32> {
    for i in 0..array.len() {
        for j in 0..array.len() - i - 1 {
            if array[j + 1] < array[j] {
                array.swap(j, j + 1);
            }
        }
    }
    array
}

pub fn bubble_sort_allocate(start: i32, sum: usize) -> i32 {
    let to_sort = allocate_array_reverse(start);
    let sorted = bubble_sort(to_sort);
    sorted.iter().take(sum).sum()
}

pub fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

pub fn print_env(args: &[&str]) {
    for arg in args {
        let (key, value) = match arg.split_once('=') {
            Some((key, value)) => {
                let actual_value =
                    std::env::var(key).expect("Environment variable must be present");
                assert_eq!(&actual_value, value, "Environment variable value differs");
                (key.to_owned(), actual_value)
            }
            None => {
                let value =
                    std::env::var(arg).expect("Pass-through environment variable must be present");
                (arg.to_string(), value)
            }
        };
        println!("{key}={value}");
    }
}

pub fn allocate_array_reverse(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

pub fn subprocess<I, T, U>(exe: T, args: U) -> io::Result<Output>
where
    T: AsRef<OsStr>,
    I: AsRef<OsStr>,
    U: IntoIterator<Item = I>,
{
    std::process::Command::new(exe).args(args).output()
}
