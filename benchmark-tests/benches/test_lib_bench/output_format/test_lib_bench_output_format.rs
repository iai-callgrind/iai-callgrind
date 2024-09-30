use std::hint::black_box;

use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EntryPoint, LibraryBenchmarkConfig,
    OutputFormat, Tool, ValgrindTool,
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .truncate_description(None)
            .show_intermediate(true)
            .show_grid(true)
        )
)]
#[bench::for_comparison(
    "Another very long string to see if the truncation is disabled with the formatting option"
)]
fn bench_with_format(_: &str) -> Vec<u64> {
    println!("Benchmark with formatting options");
    black_box(benchmark_tests::find_primes_multi_thread(3))
}

#[library_benchmark]
#[bench::for_comparison(
    "A very long string to see if the truncation of the description is really working"
)]
fn bench_without_format(_: &str) -> Vec<u64> {
    println!("Benchmark without formatting options");
    black_box(benchmark_tests::find_primes_multi_thread(2))
}

library_benchmark_group!(
    name = my_group;
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::None)
        .callgrind_args([
            "--toggle-collect=benchmark_tests::find_primes_multi_thread",
            "--toggle-collect=benchmark_tests::find_primes"
        ])
        .tool(Tool::new(ValgrindTool::DHAT));
    compare_by_id = true;
    benchmarks = bench_without_format, bench_with_format
);

main!(library_benchmark_groups = my_group);
