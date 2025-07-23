use benchmark_tests::{bubble_sort, leak_memory, setup_worst_case_array, subprocess};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Helgrind, LibraryBenchmarkConfig, Massif,
    Memcheck, ValgrindTool,
};

#[library_benchmark(setup = setup_worst_case_array)]
#[bench::memcheck_xtree(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args(["xtree-memory=full"]))
)]
#[bench::memcheck_xleak_when_no_leak(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args(["xtree-leak=yes"]))
)]
#[bench::memcheck_xtree_and_xleak_when_no_leak(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args(["xtree-memory=full", "xtree-leak=yes"]))
)]
#[bench::helgrind(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Helgrind)
        .tool(Helgrind::with_args(["xtree-memory=full"]))
)]
#[bench::massif(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Massif)
        .tool(Massif::with_args(["xtree-memory=full"]))
)]
fn bench_with_xtree(array: Vec<i32>) -> Vec<i32> {
    bubble_sort(array)
}

#[library_benchmark]
#[bench::xtree(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args([
            "xtree-memory=full", "--error-exitcode=0"
        ]))
)]
#[bench::xleak(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args([
            "xtree-leak=yes", "--error-exitcode=0"
        ]))
)]
#[bench::xtree_and_xleak(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args([
            "xtree-memory=full", "xtree-leak=yes", "--error-exitcode=0"
        ]))
)]
fn bench_with_memcheck_when_leak() {
    leak_memory(100_000);
}

#[library_benchmark]
#[bench::memcheck_multi_process(
    args = ("2"),
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Memcheck)
        .tool(Memcheck::with_args(["xtree-memory=full", "xtree-leak=yes"]))
)]
fn bench_with_xtree_in_subprocess(num_threads: &str) -> std::io::Result<std::process::Output> {
    subprocess(env!("CARGO_BIN_EXE_leak-memory"), [num_threads])
}

library_benchmark_group!(
    name = my_group;
    config = LibraryBenchmarkConfig::default()
       .valgrind_args(["enable-debuginfod=no"]);
    benchmarks =
        bench_with_xtree,
        bench_with_xtree_in_subprocess,
        bench_with_memcheck_when_leak
);
main!(library_benchmark_groups = my_group);
