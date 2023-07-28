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
    // run = cmd = "iai-callgrind-runner", args= ["0.4.0"];
    run = cmd = "/usr/bin/echo", args = ["woo", "hoo"], args = ["yippie", "yeah", "yeah", "pig"]
);
