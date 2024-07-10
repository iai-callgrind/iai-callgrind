use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

fn setup_two_arguments(first: u64, second: u64) -> u64 {
    first + second
}

fn setup_one_argument(value: u64) -> u64 {
    value * value
}

fn teardown(tuple: (u64, u64)) {
    let (result, expected) = tuple;
    if result != expected {
        panic!("Expected: {expected} but result was {result}");
    }
}

#[library_benchmark]
#[bench::with_setup(args = (2, 3), setup = setup_two_arguments)]
#[bench::with_setup_first(setup = setup_two_arguments, args = (2, 3))]
#[bench::with_teardown(args = (5), teardown = teardown)]
fn bench(value: u64) -> (u64, u64) {
    black_box((black_box(value * value), black_box(5)))
}

#[library_benchmark]
#[benches::with_setup_one_argument(
    args = [2,
           setup_one_argument(5),
           { let mut result = 0; for i in [2, 3] { result += i}; result }
    ], setup = setup_one_argument)]
fn benches_one_argument(value: u64) -> (u64, u64) {
    black_box((black_box(value * value), black_box(5)))
}

#[library_benchmark]
#[benches::with_setup(args = [(2, 3)], setup = setup_two_arguments)]
fn benches_two_arguments(value: u64) -> (u64, u64) {
    black_box((black_box(value * value), black_box(5)))
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench, benches_one_argument, benches_two_arguments
);

main!(library_benchmark_groups = bench_fibonacci_group);
