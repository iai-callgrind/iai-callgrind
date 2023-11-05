use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, Run, Tool, ValgrindTool,
};

#[inline(never)]
pub fn run_before() {
    std::fs::write("before", b"before").unwrap();
}

#[inline(never)]
pub fn run_after() {
    let string = std::fs::read_to_string("before").unwrap();
    assert_eq!(string, "before");
}

binary_benchmark_group!(
    name = subprocess;
    before = run_before, bench = true;
    after = run_after;
    benchmark = |"benchmark-tests-subprocess", group: &mut BinaryBenchmarkGroup| {
        group
            .bench(Run::with_arg(
                Arg::new(
                    "trace_sort_4000_sum_2000",
                    [
                        env!("CARGO_BIN_EXE_benchmark-tests-sort"),
                        &4000.to_string(),
                        &2000.to_string()
                    ],
                ),
            )
            .tool_override(Tool::new(ValgrindTool::DHAT)
                .args(["trace-children=yes"])
                .outfile_modifier("%p")
            )
        )
    }
);

binary_benchmark_group!(
    name = sort;
    before = run_before;
    after = run_after, bench = true;
    benchmark = |"benchmark-tests-sort", group: &mut BinaryBenchmarkGroup| {
        group
            .bench(Run::with_arg(
                Arg::new(
                    "sort_10_sum_10",
                    [
                        &10.to_string(),
                        &10.to_string()
                    ],
                ),
            ))
            .bench(Run::with_arg(
                Arg::new(
                    "sort_4000_sum_2000",
                    [
                        &4000.to_string(),
                        &2000.to_string()
                    ],
                ),
            )
        )
    }
);

// The main macro which creates a benchmarking harness with all group names from the above
// `binary_benchmark_group!` macros.
//
// We configure the regression checks for all benchmark groups with a percentage limit of `1%` for
// `Ir` (total instructions executed) and `10%` for `EstimatedCycles`. We also want to see all
// performance regressions and set `fail_fast` to false explicitly (This wouldn't have been
// necessary because the default is `false`). The whole benchmark still fails in the end if a
// performance regression was detected.
main!(
    config = BinaryBenchmarkConfig::default()
        .tool(Tool::new(ValgrindTool::DHAT))
        .tool(Tool::new(ValgrindTool::Massif))
        .tool(Tool::new(ValgrindTool::BBV))
        .tool(Tool::new(ValgrindTool::Memcheck))
        .tool(Tool::new(ValgrindTool::DRD))
        .tool(Tool::new(ValgrindTool::Helgrind));
    binary_benchmark_groups = sort, subprocess,
);
