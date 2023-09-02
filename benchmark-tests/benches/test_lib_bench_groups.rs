//! This is an example for setting up library benchmarks with the new api. An example for the old
//! api is `test_lib_bench_with_skip_setup`. It's best to read all the comments from top to bottom
//! to get a better understanding of the api.
//!
//! The new api has a lot of advantages especially handling of benchmark setup costs is greatly
//! simplified.

// These two functions from the benchmark-tests library serve as functions we want to benchmark
use benchmark_tests::{bubble_sort, fibonacci};
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};

// This function is used to create a worst case array we want to sort with our implementation of
// bubble sort
fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).collect()
    } else {
        (0..start).rev().collect()
    }
}

// This function is used to create a best case array we want to sort with our implementation of
// bubble sort
fn setup_best_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).collect()
    }
}

// The #[library_benchmark] attribute let's you define a benchmark function which you can later use
// in the `library_benchmark_groups!` macro. Just using the #[library_benchmark] attribute as a
// standalone is fine for simple function calls without parameters. However, we actually want to
// benchmark cases which would need to setup a vector with more elements, but everything we setup
// within the benchmark function itself is attributed to the event counts. See the next benchmark
// `bench_bubble_sort` function for a better example which uses the `bench` attribute to setup
// benchmark with different vectors.
#[library_benchmark]
// If possible, it's best to return something from a benchmark function
fn bench_bubble_sort_empty() -> Vec<i32> {
    // The `black_box` is needed to tell the compiler to not optimize what's inside the black_box or
    // else the benchmarks might return inaccurate results.
    black_box(bubble_sort(black_box(vec![])))
}

// This benchmark uses the `bench` attribute to setup benchmarks with different setups. The big
// advantage is, that the setup costs and event counts aren't attributed to the benchmark (and
// opposed to the old api we don't have to deal with callgrind arguments, toggles, ...)
//
// The `bench` attribute consist of the attribute name itself, an unique id after `::` and
// optionally arguments with expressions which are passed to the benchmark function as parameter.
// Here we pass a single argument with `Vec<i32>` type to the benchmark. All arguments are already
// wrapped in a black box and don't need to be put in a `black_box` again.
#[library_benchmark]
// This bench is setting up the same benchmark case as above in the `bench_bubble_sort_empty` with
// the advantage that the setup costs for creating a vector (even if it is empty) aren't attributed
// to the benchmark and that the `array` is already wrapped in a black_box.
#[bench::empty(vec![])]
// Some other use cases to play around with
#[bench::worst_case_6(vec![6, 5, 4, 3, 2, 1])]
#[bench::best_case_6(vec![1, 2, 3, 4, 5, 6])]
#[bench::best_case_20(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20])]
// Function calls are fine too
#[bench::worst_case_4000(setup_worst_case_array(4000))]
#[bench::best_case_4000(setup_best_case_array(4000))]
// The argument of the benchmark function defines the type of the argument from the `bench` cases.
fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
    // Note `array` is not put in a `black_box` because that's already done for you.
    black_box(bubble_sort(array))
}

// This benchmark serves as an example for a benchmark function having more than one argument
// (Actually, to benchmark the fibonacci function, a single argument would have been sufficient)
#[library_benchmark]
// Any expression is allowed as argument
#[bench::fib_5_plus_fib_10(255 - 250, 10)]
#[bench::fib_30_plus_fib_20(30, 20)]
fn bench_fibonacci_sum(first: u64, second: u64) -> u64 {
    black_box(black_box(fibonacci(first)) + black_box(fibonacci(second)))
}

// Use the `benchmarks` argument of the `library_benchmark_group!` macro to collect all benchmarks
// you want to put into the same group. The `name` is a unique identifier which is used in the
// `main!` macro to collect all `library_benchmark_groups`.
//
// Although not used here, this macro also accepts a `config` argument, which itself accepts a
// `LibraryBenchmarkConfig`. See the docs of `iai_callgrind` for more details about the
// `LibraryBenchmarkConfig`.
library_benchmark_group!(
    name = bubble_sort;
    benchmarks = bench_bubble_sort_empty, bench_bubble_sort
);

// In our example file here, we could have put `bench_fibonacci` into the same group as the bubble
// sort benchmarks and using a separate group merely serves as an example for having multiple
// groups.
//
// However, having separate groups can help organizing your benchmarks. The different groups are
// shown separately in the output of the callgrind run and the output files of a callgrind run are
// put in separate folders for each group.
library_benchmark_group!(
    name = fibonacci;
    benchmarks = bench_fibonacci_sum
);

// Finally, the mandatory main! macro which collects all `library_benchmark_groups`. The main! macro
// creates a benchmarking harness and runs all the benchmarks defined in the groups and benches.
main!(library_benchmark_groups = bubble_sort, fibonacci);
