use std::path::PathBuf;

use iai_callgrind::main;

const FILE_WITH_CONTENT: &str = "fixtures/file_with_content.txt";

/// This method is run once before all benchmark
#[inline(never)] // required
fn before() {
    let file = PathBuf::from(FILE_WITH_CONTENT);
    assert!(
        file.exists(),
        "before: Fixture '{}' does not exist",
        FILE_WITH_CONTENT
    );
}

/// This method is run once after all benchmarks
#[inline(never)] // required
fn after() {
    let file = PathBuf::from(FILE_WITH_CONTENT);
    assert!(
        file.exists(),
        "after: Fixture '{}' does not exist",
        FILE_WITH_CONTENT
    );
}

/// This method is run before any benchmark
#[inline(never)] // required
fn setup() {
    let file = PathBuf::from(FILE_WITH_CONTENT);
    assert!(
        file.exists(),
        "setup: Fixture '{}' does not exist",
        FILE_WITH_CONTENT
    );
}

/// This method is run after any benchmark
#[inline(never)] // required
fn teardown() {
    let file = PathBuf::from(FILE_WITH_CONTENT);
    assert!(
        file.exists(),
        "teardown: Fixture '{}' does not exist",
        FILE_WITH_CONTENT
    );
}

main!(
    before = before;
    after = after;
    setup = setup, bench = true;
    teardown = teardown;
    fixtures = "fixtures";
    run = cmd = "benchmark-tests-cat", id = "benchmark-tests-cat with content", args = ["fixtures/file_with_content.txt"];
    run = cmd = "cat", id = "cat with content", args = ["fixtures/file_with_content.txt"]
);
