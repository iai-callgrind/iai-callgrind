mod my_lib {
    pub use benchmark_tests::bubble_sort;
}
use std::hint::black_box;

use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Dhat, LibraryBenchmarkConfig,
};

#[library_benchmark]
#[bench::worst_case_3(vec![3, 2, 1])]
fn bench_library(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default());
    library_benchmark_groups = my_group
);
