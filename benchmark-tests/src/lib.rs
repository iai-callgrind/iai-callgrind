use std::ffi::OsStr;
use std::io;
use std::process::Output;

// This function is used to create a worst case array we want to sort with our implementation of
// bubble sort
pub fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

// This function is used to create a best case array we want to sort with our implementation of
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
