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
    run = cmd = "benchmark-tests", args = ["one", "two"], args = [];
    run = cmd = "/usr/bin/echo", args = ["one", "two"], args = ["one two three four"]
);
