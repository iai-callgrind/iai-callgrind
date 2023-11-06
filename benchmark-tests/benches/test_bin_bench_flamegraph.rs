use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, EventKind, FlamegraphConfig,
    FlamegraphKind, Run,
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
    name = main_level_flamegraph;
    before = run_before;
    after = run_after, bench = true;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group.bench(Run::with_arg(
            Arg::new("foo", ["0"]),
        ))
    }
);

binary_benchmark_group!(
    name = group_level_flamegraph;
    before = run_before, bench = true;
    after = run_after;
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
            Run::with_arg(Arg::new("all_flamegraph_kinds", ["0"]))
                .flamegraph(FlamegraphConfig::default()
                    .title("Run level flamegraph all kinds".to_owned())
                    .kind(FlamegraphKind::All)
                )
        )
        .bench(
            Run::with_arg(Arg::new("only_regular_kind", ["0"]))
                .flamegraph(FlamegraphConfig::default()
                    .title("Run level flamegraph regular kind".to_owned())
                    .kind(FlamegraphKind::Regular)
                )
        )
        .bench(
            Run::with_arg(Arg::new("only_differential_kind", ["0"]))
            .flamegraph(FlamegraphConfig::default()
                .title("Run level flamegraph differential kind".to_owned())
                .kind(FlamegraphKind::Differential)
            )
        )
        .bench(
            Run::with_arg(Arg::new("none_kind", ["0"]))
            .flamegraph(FlamegraphConfig::default()
                .title("Run level flamegraph no kind".to_owned())
                .kind(FlamegraphKind::None)
            )
        )
    }
);

binary_benchmark_group!(
    name = flamegraph_configurations;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| {
        group
        .bench(
            Run::with_arg(Arg::new("Ir event type", ["0"]))
            .flamegraph(
                FlamegraphConfig::default()
                .event_kinds([EventKind::Ir])
            )
        )
        .bench(
            Run::with_arg(Arg::new("no_event_kinds_then_no_flamegraph", ["0"]))
            .flamegraph(
                FlamegraphConfig::default().event_kinds([])
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
    config =
        BinaryBenchmarkConfig::default()
            .flamegraph(FlamegraphConfig::default()
                .title("Main config flamegraph".to_owned())
            );
    binary_benchmark_groups =
        main_level_flamegraph,
        group_level_flamegraph,
        run_level_flamegraph,
        flamegraph_configurations,
);
