//! This benchmark is primarily written to test `iai-callgrind` but is also documented to serve as
//! an example for setting up a binary benchmark with the builder api

use std::path::PathBuf;

use iai_callgrind::{
    binary_benchmark_group, main, Arg, BenchmarkId, BinaryBenchmarkConfig, BinaryBenchmarkGroup,
    EventKind, ExitWith, Fixtures, RegressionConfig, Run,
};

/// This function is run before all benchmarks of a group if the group argument `before =
/// run_before` is set
///
/// If running one of `before`, `after`, `setup` or `teardown` functions and `sandbox = true` (the
/// default) then these functions (like all benchmarks) are executed inside the root directory
/// of the sandbox.
///
/// The `inline(never)` is strictly required if bench = true otherwise serves to separate this
/// function from the other functions
#[inline(never)]
pub fn run_before() {
    std::fs::write("before", b"before").unwrap();
}

/// This function is run after all benchmark of a group if the group argument `after = run_after`
/// is set
#[inline(never)]
pub fn run_after() {
    let string = std::fs::read_to_string("before").unwrap();
    assert_eq!(string, "before");
}

/// This function is run before any benchmark of a group if the group argument `setup = run_setup`
/// is set
#[inline(never)]
pub fn run_setup() {
    assert!(PathBuf::from("before").exists());

    std::fs::write("setup", b"setup").unwrap();
}

/// This function is run after any benchmark of a group if the group argument `teardown =
/// run_teardown` is set
#[inline(never)]
pub fn run_teardown() {
    assert!(PathBuf::from("before").exists());

    let string = std::fs::read_to_string("setup").unwrap();
    assert_eq!(string, "setup");
}

// This group benchmarks the binary `benchmark-tests-echo` of this crate.
//
// Only `name` and `benchmark` are mandatory, the `before`, `after` etc. arguments are optional.
//
// If you don't want to specify a command for the whole group see the `group_without_cmd` group for
// an example on how to do that. However, the auto discovery of a crate's binary only works when
// specified in the `benchmark` argument and not in `Run::with_cmd` and alike.
//
// It's possible to benchmark any of `before`, `after`, `setup` or `teardown` by setting
// `bench = true`. If this argument is not given explicitly, the default is `bench = false`.
binary_benchmark_group!(
    name = group_with_cmd;
    before = run_before, bench = true;
    after = run_after, bench = true;
    setup = run_setup;
    teardown = run_teardown;
    // The fixtures directory will be copied into the root of the sandbox (as
    // `test_bin_bench_groups.fixtures`)
    config = BinaryBenchmarkConfig::default()
        .fixtures(Fixtures::new("benches/test_bin_bench_groups.fixtures"));
    benchmark = |"benchmark-tests-echo", group: &mut BinaryBenchmarkGroup| setup_echo_group(group));

/// This function is setting up the `group_with_cmd` group from above
///
/// Each `Arg` has a mandatory `id` which must be unique within the same group
fn setup_echo_group(group: &mut BinaryBenchmarkGroup) {
    group
        // We're overwriting the group command here (although using the same binary in the end)
        // because auto-discovery of a crate's binary doesn't work with `Run::with_cmd`
        .bench(Run::with_cmd(
            env!("CARGO_BIN_EXE_benchmark-tests-echo"),
            Arg::new(
                "foo",
                [
                    "foo",
                    "test_bin_bench_groups.fixtures/benchmark-tests-echo.foo.txt",
                ],
            ),
        ))
        .bench(Run::with_arg(Arg::new(
            "foo bar",
            [
                "foo bar",
                "test_bin_bench_groups.fixtures/benchmark-tests-echo.foo bar.txt",
            ],
        )))
        .bench(Run::with_arg(Arg::new(
            "foo.foo bar",
            [
                "foo",
                "foo bar",
                "test_bin_bench_groups.fixtures/benchmark-tests-echo.foo.foo bar.txt",
            ],
        )))
        .bench(
            Run::with_arg(Arg::new(
                "foo bar@current",
                ["foo bar", "benchmark-tests-echo.foo bar.txt"],
            ))
            .arg(Arg::new(
                "foo.foo bar@current",
                ["foo", "foo bar", "benchmark-tests-echo.foo.foo bar.txt"],
            )).current_dir("test_bin_bench_groups.fixtures")
        )
        .bench(
            Run::with_args([
                Arg::new(
                    "foo bar@entry",
                    ["foo bar", "benchmark-tests-echo.foo bar.txt"],
                ),
                Arg::new(
                    "foo.foo bar@entry",
                    ["foo", "foo bar", "benchmark-tests-echo.foo.foo bar.txt"],
                ),
            ])
            .current_dir("test_bin_bench_groups.fixtures")
            .entry_point("benchmark_tests_echo::main")
            .regression(RegressionConfig::default().fail_fast(true))
        );
}

// It's not necessary to set up the group within a separate function but an extra function is maybe
// more convenient to work with. Note there's also no default binary specified in the `benchmark`
// argument. The `Run::with_cmd` or `Run::with_cmd_args` methods have to be used in such a case or
// else (for example `Run::with_arg`) running this benchmark fails with an error since no command
// was specified.
binary_benchmark_group!(
    name = group_without_cmd;
    before = run_before;
    after = run_after;
    setup = run_setup, bench = true;
    teardown = run_teardown, bench = true;
    config = BinaryBenchmarkConfig::default()
        .fixtures(Fixtures::new("benches/test_bin_bench_groups.fixtures"));
    benchmark = |group: &mut BinaryBenchmarkGroup| {
        group
            // Auto-discovery of a crate's binary doesn't work with `Run::with_cmd` so we need to
            // use `env!("CARGO_BIN_EXE_name")`
            .bench(Run::with_cmd(
                env!("CARGO_BIN_EXE_benchmark-tests-echo"),
                Arg::new(
                    "foo",
                    [
                        "foo",
                        "test_bin_bench_groups.fixtures/benchmark-tests-echo.foo.txt",
                    ],
                ),
            ))
    }
);

fn run_test_env_group(group: &mut BinaryBenchmarkGroup) {
    group
        // .sandbox(false)
        .bench(
            Run::with_arg(
                    Arg::new("foo=bar", ["FOO=BAR"])
                )
                .env("FOO", "BAR").env_clear(true)
        )
        .bench(
            Run::with_arg(
                    Arg::new("does not exist", ["NOT=EXIST"])
                )
                .arg(Arg::new("home does not exist", ["HOME"]))
                .exit_with(ExitWith::Failure)
        )
        .bench(
            Run::with_arg(
                        Arg::new("home", ["HOME"])
                    )
                    .arg(Arg::new("pwd", ["PWD"]))
                    .env_clear(false).exit_with(ExitWith::Success)
        );
}

// A minimal group without the `before` ... functions which is running the crate's binary
// `benchmark-tests-printenv` using the `run_test_env_group` function from above
binary_benchmark_group!(
    name = group_test_env;
    benchmark = |"benchmark-tests-printenv", group: &mut BinaryBenchmarkGroup|
        run_test_env_group(group)
);

// Here's a small example for how to make use of the parameter of `BenchmarkId` to create unique ids
// for each `Arg`
binary_benchmark_group!(
    name = group_test_benchmark_id;
    benchmark = |"benchmark-tests-printenv", group: &mut BinaryBenchmarkGroup| {
        for i in 0..10 {
            let env = format!("MY_ENV={i}");
            group.bench(
                Run::with_arg(
                    Arg::new(BenchmarkId::new("printenv", i), [&env])
                )
                .env("MY_ENV", i.to_string()));
        }
    }
);

// The main macro which creates a benchmarking harness with all group names from the above
// `binary_benchmark_group!` macros
main!(
    config = BinaryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 1.0), (EventKind::EstimatedCycles, 10.0)])
                .fail_fast(true)
        );
    binary_benchmark_groups = group_with_cmd,
    group_without_cmd,
    group_test_env,
    group_test_benchmark_id
);
