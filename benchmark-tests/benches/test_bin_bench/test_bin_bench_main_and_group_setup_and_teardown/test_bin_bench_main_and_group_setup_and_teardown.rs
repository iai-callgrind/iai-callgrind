use std::fs::File;
use std::io::Write;

use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

const GROUP_SETUP_FILE: &str = "/tmp/iai-callgrind.group_setup.tmp";
const READ_FILE: &str = env!("CARGO_BIN_EXE_read-file");
const FILE_EXISTS: &str = env!("CARGO_BIN_EXE_file-exists");

#[binary_benchmark]
fn simple_bench() -> iai_callgrind::Command {
    let expected = format!("simple_group_with_setup: {GROUP_SETUP_FILE}");
    iai_callgrind::Command::new(READ_FILE)
        .arg(GROUP_SETUP_FILE)
        .arg(expected)
        .build()
}

fn group_setup() {
    println!("GROUP SETUP");
    let mut file = File::create(GROUP_SETUP_FILE).unwrap();
    write!(file, "simple_group_with_setup: {GROUP_SETUP_FILE}").unwrap();
}

fn group_teardown() {
    println!("GROUP TEARDOWN");
    std::fs::remove_file(GROUP_SETUP_FILE).unwrap();
}

#[binary_benchmark]
fn check_file_exists() -> iai_callgrind::Command {
    iai_callgrind::Command::new(FILE_EXISTS)
        .arg(GROUP_SETUP_FILE)
        .arg("true")
        .build()
}

#[binary_benchmark]
fn check_file_not_exists() -> iai_callgrind::Command {
    iai_callgrind::Command::new(FILE_EXISTS)
        .arg(GROUP_SETUP_FILE)
        .arg("false")
        .build()
}

binary_benchmark_group!(
    name = simple_group_with_setup;
    setup = group_setup();
    teardown = group_teardown();
    benchmarks = simple_bench, check_file_exists
);

binary_benchmark_group!(
    name = check_group;
    benchmarks = check_file_not_exists
);

fn main_setup() {
    println!("MAIN SETUP");
}

fn main_teardown() {
    println!("MAIN TEARDOWN");
}

main!(
    setup = main_setup();
    teardown = main_teardown();
    binary_benchmark_groups =
        simple_group_with_setup,
        check_group
);
