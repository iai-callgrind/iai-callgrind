use iai_callgrind::main;

/// This method is run once before any binary benchmark
#[inline(never)] // required
fn before() {
    println!("before");
}

/// This method is run once after all binary benchmarks
#[inline(never)] // required
fn after() {
    println!("after")
}

/// This method is run before a binary benchmark
#[inline(never)] // required
fn setup() {
    println!("setup")
}

/// This method is run after a binary benchmark
#[inline(never)] // required
fn teardown() {
    println!("teardown");
}

main!(
    // `before`, `after`, `setup` and `teardown` are optional arguments to run a function before or
    // after a benchmark. Specify the optional `bench` argument to benchmark the `before`, `after`,
    // `setup` or `teardown` function. If `bench` is not specified, the default is `false`.
    before = before, bench = false;
    after = after;
    setup = setup, bench = true;
    teardown = teardown, bench = true;
    run = cmd = "benchmark-tests", args = [];
    run = cmd = "benchmark-tests", args = ["one", "two"];
    run = cmd = "benchmark-tests", args = ["one", "two"], args = ["three", "four"];
    run = cmd = "benchmark-tests", args = ["test", "entry_point", "0"];
    run = cmd = "benchmark-tests", opts = Options::default().entry_point("benchmark_tests::main"), args = ["test", "entry_point", "1"];
    // command with an absolute path
    run = cmd = "/bin/echo", args = ["one", "two"], args = ["one two three four"];
    // commands in the PATH are ok too
    run = cmd = "echo", args = ["one", "two"], args = ["one two three four"];
    run = cmd = "echo",
        opts = Options::new().current_dir(PathBuf::from("/tmp")),
        args = ["one", "two"],
        args = ["one two three four"];
    run = cmd = "printenv", opts = Options::default().env_clear(false), args = ["PATH"];
    run = cmd = "printenv", envs = ["PATH"], opts = Options::default().env_clear(true), args = ["PATH"];
    run = cmd = "printenv", envs = ["HELLO=WORLD"], opts = Options::default().env_clear(true), args = ["HELLO"];
);
