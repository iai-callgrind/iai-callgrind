use iai_callgrind::{
    binary_benchmark_group, main, Arg, BenchmarkId, BinaryBenchmarkGroup, ExitWith, Run,
};

fn setup_group_tests_exits(group: &mut BinaryBenchmarkGroup) {
    group.bench(Run::with_arg(Arg::new("succeed", ["0"])).exit_with(ExitWith::Success));
    for i in 1..=3 {
        group.bench(
            Run::with_arg(Arg::new(BenchmarkId::new("fail_with", i), [i.to_string()]))
                .exit_with(ExitWith::Failure),
        );
    }
    group.bench(Run::with_arg(Arg::new("fail_with_255", ["255"])).exit_with(ExitWith::Failure));
    for i in 0..=3 {
        group.bench(
            Run::with_arg(Arg::new(BenchmarkId::new("code", i), [i.to_string()]))
                .exit_with(ExitWith::Code(i)),
        );
    }
    group.bench(Run::with_arg(Arg::new("code_255", ["-1"])).exit_with(ExitWith::Code(255)));
}

binary_benchmark_group!(
    name = test_exits;
    benchmark = |"benchmark-tests-exit", group: &mut BinaryBenchmarkGroup| setup_group_tests_exits(group)
);

main!(binary_benchmark_groups = test_exits);
