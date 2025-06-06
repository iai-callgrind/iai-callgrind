use std::hint::black_box;

use benchmark_tests as my_lib;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EventKind, LibraryBenchmarkConfig,
    RegressionConfig,
};

#[library_benchmark]
#[bench::worst_case(vec![3, 2, 1])]
fn bench_library(data: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(data))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);

main!(
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = my_group
);
