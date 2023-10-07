use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, FlamegraphConfig, LibraryBenchmarkConfig,
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

#[library_benchmark]
#[bench::worst_case(
    args = (setup_worst_case_array(10)),
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default().title("Bench flamegraph".to_owned())
            )
)]
fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
    benchmark_tests::bubble_sort(array)
}

#[library_benchmark(
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default().title("Library benchmark flamegraph".to_owned())
            )
)]
fn without_bench_attribute() -> Vec<i32> {
    benchmark_tests::bubble_sort(vec![])
}

#[library_benchmark]
#[bench::worst_case(setup_worst_case_array(10))]
fn main_level_flamegraph_config(array: Vec<i32>) -> Vec<i32> {
    benchmark_tests::bubble_sort(array)
}

#[library_benchmark]
fn function_with_many_stacks() {
    println!("Hello World!");
}

library_benchmark_group!(
    name = benches;
    benchmarks = bench_bubble_sort, without_bench_attribute, main_level_flamegraph_config, function_with_many_stacks
);

#[library_benchmark]
#[bench::fibonacci(5)]
fn recursive_function(n: u64) -> u64 {
    benchmark_tests::fibonacci(n)
}

library_benchmark_group!(
    name = recursive;
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
                FlamegraphConfig::default().title("Group level flamegraph".to_owned())
            );
    benchmarks = recursive_function
);

main!(
    config = LibraryBenchmarkConfig::default()
        .flamegraph(
            FlamegraphConfig::default().title("Main level flamegraph".to_owned())
        );
    library_benchmark_groups = benches, recursive);
