use std::hint::black_box;

/// See issue https://github.com/gungraun/gungraun/issues/198
/// Generic bench arguments cause compilation failure
///
/// After the fix the benchmark should now compile
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

#[derive(Debug)]
struct A;

fn input_a() -> A {
    A
}

#[derive(Debug)]
struct B;

fn input_b() -> B {
    B
}

fn run_format(input: impl std::fmt::Debug) -> usize {
    format!("{input:?}").len()
}

#[library_benchmark]
#[bench::a(input_a())]
#[bench::b(input_b())]
fn bench_format<I: std::fmt::Debug>(input: I) -> usize {
    black_box(run_format(input))
}

library_benchmark_group!(
    name = bench_format_group;
    benchmarks = bench_format
);

main!(library_benchmark_groups = bench_format_group);
