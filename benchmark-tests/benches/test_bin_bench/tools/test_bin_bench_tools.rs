use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Command, OutputFormat,
    Tool, ValgrindTool,
};

#[binary_benchmark]
#[bench::trace_children()]
#[bench::no_trace_children(
    config = BinaryBenchmarkConfig::default()
        .valgrind_args(["trace-children=no"])
)]
fn bench_subprocess() -> Command {
    Command::new(env!("CARGO_BIN_EXE_subprocess"))
        .arg(env!("CARGO_BIN_EXE_sort"))
        .build()
}

binary_benchmark_group!(
    name = bench_group;
    benchmarks = bench_subprocess
);

main!(
    config = BinaryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .show_intermediate(true)
            .truncate_description(None)
        )
        .tool(Tool::new(ValgrindTool::DHAT).args(["--time-stamp=yes"]))
        .tool(Tool::new(ValgrindTool::Massif))
        .tool(Tool::new(ValgrindTool::BBV))
        .tool(Tool::new(ValgrindTool::Memcheck).args(["--time-stamp=yes"]))
        .tool(Tool::new(ValgrindTool::DRD))
        .tool(Tool::new(ValgrindTool::Helgrind));
    binary_benchmark_groups = bench_group
);
