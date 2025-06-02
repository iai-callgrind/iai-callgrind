use std::hint::black_box;

use benchmark_tests::bubble_sort;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, FlamegraphConfig, LibraryBenchmarkConfig,
    RegressionConfig,
};

fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark(config = LibraryBenchmarkConfig::default().callgrind_args(["--cache-sim=yes"]))]
#[bench::with_10(setup_worst_case_array(20))]
fn bench_with_cache_sim(value: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(value))
}

#[library_benchmark(config = LibraryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]))]
#[bench::with_10(setup_worst_case_array(20))]
fn bench_without_cache_sim(value: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(value))
}

library_benchmark_group!(
    name = bench_cache_sim;
    benchmarks = bench_with_cache_sim, bench_without_cache_sim
);

main!(
    config = LibraryBenchmarkConfig::default()
        .regression(RegressionConfig::default().fail_fast(true))
        .flamegraph(FlamegraphConfig::default());
    library_benchmark_groups = bench_cache_sim);
