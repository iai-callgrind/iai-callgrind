use rstest::rstest;

use crate::common;

#[rstest]
#[case::major("major")]
#[case::minor("minor")]
#[case::patch("patch")]
fn test_library_version_newer_than_runner_version(#[case] part: &str) {
    let runner_version = common::get_runner_version();
    let library_version = {
        let mut library_version = runner_version.clone();
        library_version.increment(part);
        library_version
    };

    let expected_stderr = format!(
        "iai_callgrind_runner: Error: iai-callgrind-runner ({runner_version}) is older than \
         iai-callgrind ({library_version}). Please update iai-callgrind-runner by calling 'cargo \
         install --version {library_version} iai-callgrind-runner'\n"
    );

    common::Runner::new()
        .args(&[&library_version.to_string()])
        .run()
        .assert_stderr_bytes(expected_stderr.as_bytes())
        .assert_stdout_is_empty();
}

// We still error out here because we don't supply the rest of the necessary arguments
#[test]
fn test_library_version_equals_runner_version() {
    let version = common::get_runner_version();
    let expected_stderr = format!(
        "iai_callgrind_runner: Error: Failed to initialize iai-callgrind-runner: Unexpected \
         number of arguments\n\nDetected version of iai-callgrind-runner is {version}. This error \
         can be caused by a version mismatch between iai-callgrind and iai-callgrind-runner. If \
         you updated the library (iai-callgrind) in your Cargo.toml file, the binary \
         (iai-callgrind-runner) needs to be updated to the same version and vice versa.\n"
    );

    common::Runner::new()
        .args(&[&version.to_string()])
        .run()
        .assert_stderr_bytes(expected_stderr.as_bytes())
        .assert_stdout_is_empty();
}

// This can happen with versions of `iai-callgrind` < 0.3.0 because we don't submit the library
// version as first argument
#[test]
fn test_library_version_not_submitted() {
    let runner_version = common::get_runner_version();
    let expected_stderr = format!(
        "iai_callgrind_runner: Error: No version information found for iai-callgrind but \
         iai-callgrind-runner ({runner_version}) is >= '0.3.0'. Please update iai-callgrind to \
         '{runner_version}' in your Cargo.toml file\n"
    );

    common::Runner::new()
        .args(&["no version"])
        .run()
        .assert_stderr_bytes(expected_stderr.as_bytes())
        .assert_stdout_is_empty();
}

#[test]
fn test_library_version_older_than_runner_version() {
    let runner_version = common::get_runner_version();
    let library_version = {
        let mut library_version = runner_version.clone();
        // just to be sure we decrement at least one part because decrement saturates at 0
        library_version.decrement("major");
        library_version.decrement("minor");
        library_version.decrement("patch");
        library_version
    };

    let expected_stderr = format!(
        "iai_callgrind_runner: Error: iai-callgrind-runner ({runner_version}) is newer than \
         iai-callgrind ({library_version}). Please update iai-callgrind to '{runner_version}' in \
         your Cargo.toml file\n"
    );

    common::Runner::new()
        .args(&[&library_version.to_string()])
        .run()
        .assert_stderr_bytes(expected_stderr.as_bytes())
        .assert_stdout_is_empty();
}
