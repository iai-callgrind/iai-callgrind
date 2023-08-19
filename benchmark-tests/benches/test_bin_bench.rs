use std::path::PathBuf;

use iai_callgrind::{main, ExitWith, Options};

/// This method is run once before all benchmark
#[inline(never)] // required
fn before() {
    println!("before");
}

/// This method is run once after all benchmarks
#[inline(never)] // required
fn after() {
    println!("after")
}

/// This method is run before any benchmark
#[inline(never)] // required
fn setup() {
    println!("setup")
}

/// This method is run after any benchmark
#[inline(never)] // required
fn teardown() {
    println!("teardown");
}

main!(
    // `before`, `after`, `setup` and `teardown` are optional arguments. They specify a function
    // which is run before or after all benchmarks or to setup and teardown every benchmark. Specify
    // the optional `bench` argument to benchmark the `before`, `after`, `setup` or `teardown`
    // function. If `bench` is not specified, the default is `false`.
    before = before, bench = false;
    after = after;
    setup = setup, bench = true;
    teardown = teardown, bench = true;
    run = cmd = "benchmark-tests", id = "empty", args = [];
    run = cmd = "benchmark-tests", id = "one two", args = ["one", "two"];
    run = cmd = "benchmark-tests", id = "one two 2", args = ["one", "two"], id = "three four", args = ["three", "four"];
    run = cmd = "benchmark-tests", id = "entry point 0", args = ["test", "entry_point", "0"];
    run = cmd = "benchmark-tests", opts = Options::default().entry_point("benchmark_tests::main"), id = "entry point 1", args = ["test", "entry_point", "1"];
    // command with an absolute path
    run = cmd = "/bin/echo", id = "echo 2 args", args = ["one", "two"], id = "echo 4 words", args = ["one two three four"];
    // commands in the PATH are ok too
    run = cmd = "echo", id = "PATH echo 2 args", args = ["one", "two"], id = "PATH echo 4 words", args = ["one two three four"];
    run = cmd = "echo",
        opts = Options::default().current_dir(PathBuf::from("/tmp")),
        id = "current dir 2 args", args =  ["one", "two"],
        id = "current dir 4 words", args = ["one two three four"];
    run = cmd = env!("CARGO_BIN_EXE_benchmark-tests"), id = "env! no args", args = [];
    run = cmd = "benchmark-tests", opts = Options::default().exit_with(ExitWith::Success), id = "exit with success", args = [];
);
