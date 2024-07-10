use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

fn setup_two_arguments(first: u64, second: u64) -> u64 {
    first + second
}

fn setup_one_argument(value: u64) -> u64 {
    value * value
}

fn setup_no_argument() -> u64 {
    9
}

fn teardown((result, expected): (u64, u64)) {
    if result != expected {
        panic!("Expected: {expected} but result was {result}");
    }
}

#[library_benchmark]
#[bench::no_argument(args = (), setup = setup_no_argument)]
#[bench::one_argument(args = (3), setup = setup_one_argument)]
#[bench::two_arguments(args = (3, 6), setup = setup_two_arguments)]
#[bench::expression(args = ({
        let mut result = 0;
        for i in [2, 3] { result += i };
        result
    }), setup = setup_one_argument)]
#[bench::setup_first_then_args(setup = setup_two_arguments, args = (3, 6))]
fn bench_only_setup(value: u64) -> u64 {
    black_box(value * value)
}

#[library_benchmark]
#[bench::simple(args = (2, 3, 5), teardown = teardown)]
#[bench::teardown_first_then_args(teardown = teardown, args = (2, 3, 5))]
#[bench::with_args_expression(args = (2, 3, {
        let mut result = 0;
        for i in [2, 3] { result += i };
        result
    }), teardown = teardown)]
fn bench_only_teardown(a: u64, b: u64, c: u64) -> (u64, u64) {
    black_box((black_box(a + b), c))
}

#[library_benchmark]
#[benches::no_argument(args = [], setup = setup_no_argument)]
#[benches::with_setup_one_argument(
    args = [2,
           setup_one_argument(5),
           { let mut result = 0; for i in [2, 3] { result += i}; result }
    ], setup = setup_one_argument)]
fn benches_only_setup(value: u64) -> u64 {
    black_box(value * value)
}

#[library_benchmark]
#[benches::simple(args = [(2, 3, 5)], teardown = teardown)]
#[benches::teardown_first_then_args(teardown = teardown, args = [(2, 3, 5)])]
#[benches::with_setup_one_argument(
    args = [(2, 3, 5),
            (2, 3, { let mut result = 0; for i in [2, 3] { result += i}; result })
    ], teardown = teardown)]
fn benches_only_teardown(a: u64, b: u64, c: u64) -> (u64, u64) {
    black_box((black_box(a + b), c))
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench_only_setup, bench_only_teardown, benches_only_setup, bench_only_teardown
);

main!(library_benchmark_groups = bench_fibonacci_group);
