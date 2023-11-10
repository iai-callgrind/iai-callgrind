use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, BinaryBenchmarkGroup, Fixtures, Run,
};

fn my_setup() {
    println!("We can put code in here which will be run before each benchmark run");
}

// We specify a cmd `"benchmark-tests-exe"` for the whole group which is a
// binary of our crate. This eliminates the need to specify a `cmd` for each
// `Run` later on and we can use the auto-discovery of a crate's binary at group
// level. We'll also use the `setup` argument to run a function before each of
// the benchmark runs.
binary_benchmark_group!(
    name = my_exe_group;
    setup = my_setup;
    // This directory will be copied into the root of the sandbox (as `fixtures`)
    config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("fixtures"));
    benchmark =
        |"benchmark-tests-printargs", group: &mut BinaryBenchmarkGroup| {
            setup_my_exe_group(group)
    }
);

// Working within a macro can be tedious sometimes so we moved the setup code
// into this method
fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
    group
        // Setup our first run doing something with our fixture `test1.txt`. The
        // id (here `do foo with test1`) of an `Arg` has to be unique within the
        // same group
        .bench(Run::with_arg(Arg::new(
            "do foo with test1",
            ["--foo=fixtures/test1.txt"],
        )))

        // Setup our second run with two positional arguments. We're not
        // interested in anything happening before the main function in
        // `benchmark-tests-printargs`, so we set the entry_point.
        .bench(
            Run::with_arg(
                Arg::new(
                    "positional arguments",
                    ["foo", "foo bar"],
                )
            ).entry_point("benchmark_tests_printargs::main")
        )

        // Our last run doesn't take an argument at all.
        .bench(Run::with_arg(Arg::empty("no argument")));
}

// As last step specify all groups we want to benchmark in the main! macro
// argument `binary_benchmark_groups`. The main macro is always needed and
// finally expands to a benchmarking harness
main!(binary_benchmark_groups = my_exe_group);
