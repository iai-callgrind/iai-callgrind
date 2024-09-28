use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[library_benchmark]
#[bench::short(10)]
#[bench::long(30)]
fn bench_fibonacci(value: u64) -> u64 {
    black_box(fibonacci(value))
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench_fibonacci
);

main!(
    config = LibraryBenchmarkConfig::default().callgrind_args(["collect-jumps=yes"]);
    library_benchmark_groups = bench_fibonacci_group
);
