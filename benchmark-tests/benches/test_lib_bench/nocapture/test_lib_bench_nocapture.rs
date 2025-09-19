use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};

fn setup_to_stdout(value: u64) -> u64 {
    println!("setup: stdout: {value}");
    value + 10
}

fn setup_to_stderr(value: u64) -> u64 {
    eprintln!("setup: stderr: {value}");
    value + 20
}

fn teardown_to_stdout(value: u64) {
    println!("teardown: stdout: {value}");
}

fn teardown_to_stderr(value: u64) {
    eprintln!("teardown: stderr: {value}");
}

#[library_benchmark]
#[bench::setup_stdout_teardown_stderr(
    args = (1),
    setup = setup_to_stdout,
    teardown = teardown_to_stderr
)]
#[bench::setup_stderr_teardown_stdout(
    args = (1),
    setup = setup_to_stderr,
    teardown = teardown_to_stdout
)]
fn bench(value: u64) -> u64 {
    println!("bench: stdout: {value}");
    eprintln!("bench: stderr: {value}");
    value + black_box(100)
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench,
);

main!(library_benchmark_groups = bench_fibonacci_group);
