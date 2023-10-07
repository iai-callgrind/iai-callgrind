use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, EventType, FlamegraphConfig, Run,
};

binary_benchmark_group!(
    name = main_level_flamegraph;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group.bench(Run::with_arg(
            Arg::new("foo", ["0"]),
        ))
    }
);

binary_benchmark_group!(
    name = group_level_flamegraph;
    config = BinaryBenchmarkConfig::default()
        .flamegraph(FlamegraphConfig::default().title("Group level flamegraph".to_owned()));
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group.bench(Run::with_arg(
            Arg::new("foo", ["0"]),
        ))
    }
);

binary_benchmark_group!(
    name = run_level_flamegraph;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group.bench(
            Run::with_arg(Arg::new("foo", ["0"]))
            .flamegraph(FlamegraphConfig::default().title("Run level flamegraph".to_owned()))
        )
    }
);

binary_benchmark_group!(
    name = flamegraph_configurations;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group
        .bench(
            Run::with_arg(Arg::new("ignore_missing_event_type", ["0"]))
            .flamegraph(
                FlamegraphConfig::default()
                .event_types([EventType::SysCpuTime])
                .ignore_missing(true)
            )
        )
        .bench(
            Run::with_arg(Arg::new("no_event_types_then_no_flamegraph", ["0"]))
            .flamegraph(
                FlamegraphConfig::default().event_types([])
            )
        )
        .bench(
            Run::with_arg(Arg::new("with_entry_point", ["0"]))
            .entry_point("benchmark_tests_exit::main")
            .flamegraph(FlamegraphConfig::default())
        )
    }
);

main!(
    config = BinaryBenchmarkConfig::default().flamegraph(FlamegraphConfig::default().title("Main config flamegraph".to_owned()));
    // binary_benchmark_groups = main_level_flamegraph, group_level_flamegraph, run_level_flamegraph, flamegraph_configurations
    binary_benchmark_groups = flamegraph_configurations
);
