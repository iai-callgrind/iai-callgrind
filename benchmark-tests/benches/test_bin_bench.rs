use iai_callgrind::main;

fn before() {
    println!("before");
}

fn after() {
    println!("after")
}

fn setup() {
    println!("setup")
}

fn teardown() {
    println!("teardown");
}

main!(
    before = before;
    after = after;
    setup = setup;
    teardown = teardown;
    run = cmd = "benchmark-tests", args = [];
    run = cmd = "benchmark-tests", args = ["one", "two"];
    run = cmd = "benchmark-tests", args = ["one", "two"], args = ["three", "four"];
    run = cmd = "benchmark-tests", args = ["test", "entry_point", "0"];
    run = cmd = "benchmark-tests", opts = Options::default().entry_point("benchmark_tests::main"), args = ["test", "entry_point", "1"];
    run = cmd = "/usr/bin/echo", args = ["one", "two"], args = ["one two three four"];
    run = cmd = "/usr/bin/echo",
        opts = Options::new().current_dir(PathBuf::from("/tmp")),
        args = ["one", "two"],
        args = ["one two three four"];
    run = cmd = "/usr/bin/printenv", opts = Options::default().env_clear(false), args = [];
    run = cmd = "/usr/bin/printenv", opts = Options::default().env_clear(false), args = ["PATH"];
    run = cmd = "/usr/bin/printenv", envs = ["PATH"], opts = Options::default().env_clear(true), args = ["PATH"];
    run = cmd = "/usr/bin/printenv", envs = ["HELLO=WORLD"], opts = Options::default().env_clear(true), args = ["HELLO"];
);
