use std::path::PathBuf;

use iai_callgrind::main;

// This is a symlink in the `fixtures-with-symlinks` directory to the file_with_content.txt file
const FILE_WITH_CONTENT: &str = "fixtures-with-symlinks/file_with_content.txt";

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
    fixtures = "fixtures-with-symlinks", follow_symlinks = true;
    run = cmd = "benchmark-tests-cat", args = ["fixtures-with-symlinks/file_with_content.txt"];
    run = cmd = "cat", args = ["fixtures-with-symlinks/file_with_content.txt"]
);
