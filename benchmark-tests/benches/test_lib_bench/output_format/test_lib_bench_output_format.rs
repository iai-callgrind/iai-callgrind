use std::hint::black_box;

use benchmark_tests::{bubble_sort, setup_worst_case_array};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Cachegrind, CachegrindMetric, Callgrind,
    CallgrindMetrics, Dhat, DhatMetric, Drd, EntryPoint, ErrorMetric, EventKind, Helgrind,
    LibraryBenchmarkConfig, Memcheck, OutputFormat, ValgrindTool,
};

// TODO: ADD TESTS for miss rates and hit rates

// The --collect-systime=nsec option is not supported on freebsd and apple, so we use
// --collect-systime=yes instead on these targets
//
// --simulate-wb=yes does not work together with --cacheuse=yes, so it is excluded here
#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
pub fn config_with_all_data_collection_options() -> LibraryBenchmarkConfig {
    LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            "collect-jumps=yes",
            "collect-systime=yes",
            "collect-bus=yes",
            "cache-sim=yes",
            "branch-sim=yes",
        ]))
        .clone()
}

#[cfg(not(any(target_vendor = "apple", target_os = "freebsd")))]
pub fn base_config() -> LibraryBenchmarkConfig {
    LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            "collect-jumps=yes",
            "collect-systime=nsec",
            "collect-bus=yes",
            "cache-sim=yes",
            "branch-sim=yes",
        ]))
        .clone()
}

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

#[library_benchmark]
#[bench::implicit_default_with_wb(config = base_config()
    .tool(Callgrind::with_args(["simulate-wb=yes"]))
)]
#[bench::implicit_default_with_cacheuse(config = base_config()
    .tool(Callgrind::with_args(["cacheuse=yes"]))
)]
#[bench::explicit_default_with_wb(config =
    base_config()
        .tool(Callgrind::with_args(["simulate-wb=yes"])
            .format([CallgrindMetrics::Default])
        )
)]
#[bench::explicit_default_with_cacheuse(config =
    base_config()
        .tool(Callgrind::with_args(["cacheuse=yes"])
            .format([CallgrindMetrics::Default])
        )
)]
#[bench::all_with_wb(config =
    base_config()
        .tool(Callgrind::with_args(["simulate-wb=yes"])
            .format([CallgrindMetrics::All])
        )
)]
#[bench::all_with_cachuse(config =
    base_config()
        .tool(Callgrind::with_args(["cacheuse=yes"])
            .format([CallgrindMetrics::All])
        )
)]
#[bench::cache_misses(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::CacheMisses])
        )
)]
#[bench::cache_miss_rates(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::CacheMissRates])
        )
)]
#[bench::cache_hits(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::CacheHits])
        )
)]
#[bench::cache_hit_rates(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::CacheHitRates])
        )
)]
#[bench::cache_sim(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::CacheSim])
        )
)]
#[bench::cache_use(config =
    base_config()
        .tool(Callgrind::with_args(["cacheuse=yes"])
            .format([CallgrindMetrics::CacheUse])
        )
)]
#[bench::system_calls(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::SystemCalls])
        )
)]
#[bench::branch_sim(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::BranchSim])
        )
)]
#[bench::write_back(config =
    base_config()
        .tool(Callgrind::with_args(["simulate-wb=yes"])
            .format([
                    // Without `Ir` the counts would be all zero
                    CallgrindMetrics::SingleEvent(EventKind::Ir),
                    CallgrindMetrics::WriteBackBehaviour
                ]
            )
        )
)]
#[bench::single_event_ir(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::SingleEvent(EventKind::Ir)])
        )
)]
#[bench::single_event_ge(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::SingleEvent(EventKind::Ge)])
        )
)]
#[bench::single_event_total_rw(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::SingleEvent(EventKind::TotalRW)])
        )
)]
#[bench::single_event_estimated_cycles(config =
    base_config()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::SingleEvent(EventKind::EstimatedCycles)])
        )
)]
fn bench_with_custom_callgrind_format() -> Vec<i32> {
    println!("Hello world!");
    black_box(bubble_sort(black_box(setup_worst_case_array(black_box(
        10,
    )))))
}

#[library_benchmark]
#[bench::cachegrind_all(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Cachegrind)
        .tool(Cachegrind::with_args(["branch-sim=yes"])
            .format([
                CachegrindMetric::Ir,
                CachegrindMetric::Dr,
                CachegrindMetric::Dw,
                CachegrindMetric::I1mr,
                CachegrindMetric::D1mr,
                CachegrindMetric::D1mw,
                CachegrindMetric::ILmr,
                CachegrindMetric::DLmr,
                CachegrindMetric::DLmw,
                CachegrindMetric::I1MissRate,
                CachegrindMetric::D1MissRate,
                CachegrindMetric::LLiMissRate,
                CachegrindMetric::LLdMissRate,
                CachegrindMetric::LLMissRate,
                CachegrindMetric::L1hits,
                CachegrindMetric::LLhits,
                CachegrindMetric::RamHits,
                CachegrindMetric::TotalRW,
                CachegrindMetric::EstimatedCycles,
                CachegrindMetric::L1HitRate,
                CachegrindMetric::LLHitRate,
                CachegrindMetric::RamHitRate,
                CachegrindMetric::Bc,
                CachegrindMetric::Bcm,
                CachegrindMetric::Bi,
                CachegrindMetric::Bim,
            ])
        )
)]
#[bench::cachegrind(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Cachegrind)
        .tool(Cachegrind::default()
            .format([CachegrindMetric::I1mr, CachegrindMetric::EstimatedCycles])
        )
)]
#[bench::dhat(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DHAT)
        .tool(Dhat::default().format([DhatMetric::TotalBlocks, DhatMetric::TotalBytes]))
)]
#[bench::memcheck(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::default().format([ErrorMetric::SuppressedErrors, ErrorMetric::Errors]))
)]
#[bench::helgrind(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Helgrind)
        .tool(Helgrind::default()
            .format([ErrorMetric::Contexts, ErrorMetric::SuppressedContexts])
        )
)]
#[bench::drd(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DRD)
        .tool(Drd::default().format([ErrorMetric::Errors, ErrorMetric::Contexts]))
)]
fn bench_with_custom_format() -> Vec<i32> {
    println!("Hello world!");
    black_box(bubble_sort(black_box(setup_worst_case_array(black_box(
        10,
    )))))
}

library_benchmark_group!(
    name = my_group;
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            "--toggle-collect=benchmark_tests::find_primes_multi_thread",
            "--toggle-collect=benchmark_tests::find_primes"
            ])
            .entry_point(EntryPoint::None)
        )
        .tool(Dhat::default());
    compare_by_id = true;
    benchmarks = bench_without_format, bench_with_format
);

library_benchmark_group!(
    name = custom_format;
    benchmarks = bench_with_custom_callgrind_format, bench_with_custom_format
);

main!(library_benchmark_groups = my_group, custom_format);
