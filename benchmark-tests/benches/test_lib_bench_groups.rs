//! This is an example for setting up library benchmarks with the new api. An example for the old
//! api is `test_lib_bench_with_skip_setup`. It's best to read all the comments from top to bottom
//! to get a better understanding of the api.
//!
//! The new api has a lot of advantages especially handling of benchmark setup costs is greatly
//! simplified.

// These two functions from the benchmark-tests library serve as functions we want to benchmark
use std::hint::black_box;

use benchmark_tests::{bubble_sort, fibonacci};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EventKind, LibraryBenchmarkConfig,
    RegressionConfig, Tool, ValgrindTool,
};

// This function is used to create a worst case array we want to sort with our implementation of
// bubble sort
fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

// This function is used to create a best case array we want to sort with our implementation of
// bubble sort
fn setup_best_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).collect()
    } else {
        (0..start).collect()
    }
}

// TODO: REMOVE TESTING CODE
fn setup_test(a: i32, b: i32) -> (i32, i32) {
    (a + b, a - b)
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

// TODO: REMOVE TESTING CODE
#[library_benchmark]
// #[benches::my0((1, 2), (2, 3))]
// #[benches::my1(args = [(1, 2), (2, 3)])]
// #[benches::my0(args = [])]
// #[benches::my1(args = [(1), (2,), [3], 5])]
// #[benches::my1(args = [(1), (2,), [3], 5])]
#[benches::my1(args = [(1, 2)], setup = setup_test)]
// fn bench_bubble_sort_empty_testing(_c: i32, _d: i32) -> Vec<i32> {
fn bench_bubble_sort_empty_testing((_c, _d): (i32, i32)) -> Vec<i32> {
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
#[bench::best_case_6(vec![1, 2, 3, 4, 5, 6, 7])]
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

// It's possible to specify a `LibraryBenchmarkConfig` valid for all benches of this
// `library_benchmark`. Since we only use the default here for demonstration purposes actually
// nothing changes. The default configuration is always applied.
#[library_benchmark(config = LibraryBenchmarkConfig::default())]
fn bench_fibonacci_with_config() -> u64 {
    black_box(black_box(fibonacci(black_box(8))))
}

// A config per `bench` attribute is also possible using the alternative `bench` attribute with
// key = value pairs. The example below shows all accepted keys.
//
// Note that `LibraryBenchmarkConfig` is additive for callgrind arguments, tools and environment
// variables and appends them to the variables of `configs` of higher levels (like
// #[library_benchmark(config = ...)]). Only the last definition of a such configuration values is
// taken into account. Other non-collection like configuration values (like `RegressionConfig`) are
// overridden. In our example here: If `raw_callgrind_args(["--dump-instr=yes"])` would have been
// specified in a higher level configuration, then specifying
// `raw_callgrind_args(["--dump-instr=no")` in our configurations at this level would effectively
// overwrite the value for `--dump-instr` and only `--dump-instr=no` is applied for the benchmark
// run `fib_with_config`.
//
// Completely overriding previous definitions of valgrind tools instead of appending them with
// `LibraryBenchmarkConfig::tool` or `LibraryBenchmarkConfig::tools` can be achieved with
// `LibraryBenchmarkConfig::tool_override` or `LibraryBenchmarkConfig::tools_override`.
#[library_benchmark]
#[bench::fib_with_config(
    args = (3, 4),
    config = LibraryBenchmarkConfig::default()
        .tool_override(
            Tool::new(ValgrindTool::Massif)
        )
)]
fn bench_fibonacci_with_config_at_bench_level(first: u64, second: u64) -> u64 {
    black_box(black_box(fibonacci(black_box(first + second))))
}

// Use the `benchmarks` argument of the `library_benchmark_group!` macro to collect all benchmarks
// you want and put them into the same group. The `name` is a unique identifier which is used in the
// `main!` macro to collect all `library_benchmark_groups`.
//
// It's also possible to specify a `LibraryBenchmarkConfig` valid for all benchmarks of this
// `library_benchmark_group`. We configure the regression checks to fail the whole benchmark run as
// soon as a performance regression happens. This'll overwrite the `RegressionConfig` of the
// configuration of the `main!` macro.
library_benchmark_group!(
    name = bubble_sort;
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default().fail_fast(false)
        );
    benchmarks =
        bench_bubble_sort_empty_testing,
        bench_bubble_sort_empty,
        bench_bubble_sort,
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
    benchmarks =
        bench_fibonacci_sum,
        bench_fibonacci_with_config,
        bench_fibonacci_with_config_at_bench_level
);

// Finally, the mandatory main! macro which collects all `library_benchmark_groups` and optionally
// accepts a `config = ...` argument before the `library_benchmark_groups` argument. The main! macro
// creates a benchmarking harness and runs all the benchmarks defined in the groups and benches.
//
// We configure the regression checks to fail gracefully at the end of the whole benchmark run
// (`fail-fast = false`) using `EventKind::Ir` (Total instructions executed) with a limit of `+5%`
// and `EventKind::EstimatedCycles` with a limit of `+10%`. This `LibraryBenchmarkConfig` applies to
// all benchmarks in all groups (specified below) if it is not overwritten.
//
// In addition to running `callgrind` it's possible to run other valgrind tools like DHAT, Massif,
// (the experimental) BBV, Memcheck, Helgrind or DRD. Below we specify to run DHAT in addition to
// callgrind for all benchmarks (if not specified otherwise and/or overridden in a lower-level
// configuration). The output files of the profiling tools (DHAT, Massif, BBV) can be found next to
// the output files of the callgrind runs in `target/iai/...`.
main!(
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 5.0), (EventKind::EstimatedCycles, 10.0)])
        )
        .tool(Tool::new(ValgrindTool::DHAT));
    library_benchmark_groups = bubble_sort, fibonacci);
