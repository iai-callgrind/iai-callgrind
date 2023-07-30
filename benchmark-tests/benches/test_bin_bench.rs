use iai_callgrind::main;

#[inline(never)]
fn before() {
    println!("before");
}

#[inline(never)]
fn after() {
    println!("after")
}

#[inline(never)]
fn setup() {
    println!("setup")
}

#[inline(never)]
fn teardown() {
    println!("teardown");
}

main!(
    before = before, bench = true;
    after = after, bench = true;
    setup = setup, bench = true;
    teardown = teardown, bench = true;
    run = cmd = "benchmark-tests", args = [];
    run = cmd = "benchmark-tests", args = ["one", "two"];
    run = cmd = "benchmark-tests", args = ["one", "two"], args = ["three", "four"];
    run = cmd = "benchmark-tests", args = ["test", "entry_point", "0"];
    run = cmd = "benchmark-tests", opts = Options::default().entry_point("benchmark_tests::main"), args = ["test", "entry_point", "1"];
    run = cmd = "/bin/echo", args = ["one", "two"], args = ["one two three four"];
    run = cmd = "echo", args = ["one", "two"], args = ["one two three four"];
    run = cmd = "echo",
        opts = Options::new().current_dir(PathBuf::from("/tmp")),
        args = ["one", "two"],
        args = ["one two three four"];
    run = cmd = "printenv", opts = Options::default().env_clear(false), args = ["PATH"];
    run = cmd = "printenv", envs = ["PATH"], opts = Options::default().env_clear(true), args = ["PATH"];
    run = cmd = "printenv", envs = ["HELLO=WORLD"], opts = Options::default().env_clear(true), args = ["HELLO"];
);
