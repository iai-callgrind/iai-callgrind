use std::io;
use std::process::Output;

use benchmark_tests::{bubble_sort, bubble_sort_allocate, subprocess};
use iai_callgrind::{
    black_box, library_benchmark, library_benchmark_group, main, EventKind, LibraryBenchmarkConfig,
    RegressionConfig, Tool, ValgrindTool,
};

fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[bench::empty(vec![])]
#[bench::worst_case_4000(setup_worst_case_array(4000))]
fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(array))
}

#[library_benchmark]
fn bench_bubble_sort_allocate() -> i32 {
    bubble_sort_allocate(black_box(4000), black_box(2000))
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool_override(
            Tool::new(ValgrindTool::DHAT)
                .args(["--trace-children=yes"])
                .outfile_modifier("%p")
        )
)]
fn bench_subprocess() -> io::Result<Output> {
    println!("Do something before calling subprocess");
    subprocess(
        black_box(env!("CARGO_BIN_EXE_benchmark-tests-sort")),
        black_box(Vec::<std::ffi::OsString>::new()),
    )
}

library_benchmark_group!(
    name = bench_group;
    benchmarks = bench_bubble_sort_allocate, bench_subprocess, bench_bubble_sort
);

main!(
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 5.0), (EventKind::EstimatedCycles, 10.0)])
        )
        .tool(Tool::new(ValgrindTool::DHAT))
        .tool(Tool::new(ValgrindTool::Massif))
        .tool(Tool::new(ValgrindTool::BBV))
        .tool(Tool::new(ValgrindTool::Memcheck))
        .tool(Tool::new(ValgrindTool::DRD))
        .tool(Tool::new(ValgrindTool::Helgrind));
    library_benchmark_groups = bench_group);
