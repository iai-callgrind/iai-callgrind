use std::hint::black_box;

use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EntryPoint, LibraryBenchmarkConfig,
};

#[inline(never)]
fn nested() -> u64 {
    benchmark_tests::fibonacci(10)
}

#[inline(never)]
fn some_func() -> u64 {
    nested()
}

#[library_benchmark()]
#[bench::none(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::None)
)]
#[bench::default(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::Default)
)]
#[bench::some(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::from("test_lib_bench_entry_point::nested"))
)]
#[bench::other(
    config = LibraryBenchmarkConfig::default()
        .entry_point("test_lib_bench_entry_point::nested"),
)]
fn bench_lib() -> u64 {
    black_box(some_func())
}

library_benchmark_group!(name = my_group; benchmarks = bench_lib);
main!(library_benchmark_groups = my_group);
