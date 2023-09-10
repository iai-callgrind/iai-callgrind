use std::path::PathBuf;

use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, ExitWith, Fixtures, Run,
};

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

binary_benchmark_group!(
    name = test_fixtures_with_follow_symlinks;
    before = before;
    after = after;
    setup = setup, bench = true;
    teardown = teardown;
    config = BinaryBenchmarkConfig::default()
        .fixtures(
            Fixtures::new("fixtures-with-symlinks").follow_symlinks(true)
        );
    benchmark = |"benchmark-tests-cat", group: &mut BinaryBenchmarkGroup| {
        group.bench(
            Run::with_arg(
                Arg::new(
                    "benchmark-tests-cat with content",
                    ["fixtures-with-symlinks/file_with_content.txt"]
                )
            )
        ).bench(
            Run::with_cmd(
                "cat",
                Arg::new("cat with content", ["fixtures-with-symlinks/file_with_content.txt"])
            )
        )
    }
);

binary_benchmark_group!(
    name = test_fixtures_without_follow_symlinks;
    config = BinaryBenchmarkConfig::default()
        .fixtures(Fixtures::new("fixtures-with-symlinks").follow_symlinks(false));
    benchmark = |"benchmark-tests-cat", group: &mut BinaryBenchmarkGroup| {
        group.bench(
            Run::with_arg(
                Arg::new(
                    "benchmark-tests-cat with content",
                    ["fixtures-with-symlinks/file_with_content.txt"]
                )
            ).exit_with(ExitWith::Failure)
        ).bench(
            Run::with_cmd(
                "cat",
                Arg::new("cat with content", ["fixtures-with-symlinks/file_with_content.txt"])
            ).exit_with(ExitWith::Failure)
        )
    }
);

main!(
    binary_benchmark_groups = test_fixtures_with_follow_symlinks,
    test_fixtures_without_follow_symlinks
);
