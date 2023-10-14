use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, FlamegraphConfig, FlamegraphKind,
    LibraryBenchmarkConfig,
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
#[bench::all_kinds(
    args = (setup_worst_case_array(10)),
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default()
                   .title("Bench-level flamegraph both kinds".to_owned())
                   .kind(FlamegraphKind::All)
            )
)]
#[bench::regular_kind(
    args = (setup_worst_case_array(10)),
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default()
                   .title("Bench-level flamegraph only regular kind".to_owned())
                   .kind(FlamegraphKind::Regular)
            )
)]
#[bench::differential_kind(
    args = (setup_worst_case_array(1000)),
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default()
                   .title("Bench-level flamegraph only differential kind".to_owned())
                   .kind(FlamegraphKind::Differential)
            )
)]
#[bench::none_kind(
    args = (setup_worst_case_array(10)),
    config =
        LibraryBenchmarkConfig::default()
            .flamegraph(
               FlamegraphConfig::default()
                   .title("No bench-level flamegraph".to_owned())
                   .kind(FlamegraphKind::None)
            )
)]
fn bench_level_flamegraphs(array: Vec<i32>) -> Vec<i32> {
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
    benchmarks =
        bench_level_flamegraphs,
        without_bench_attribute,
        main_level_flamegraph_config,
        function_with_many_stacks
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
