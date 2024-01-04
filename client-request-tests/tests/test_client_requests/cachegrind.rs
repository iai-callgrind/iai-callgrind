use std::io::{stderr, Write};

use crate::common;

#[test]
fn test_cachegrind_reqs_when_running_native() {
    let mut cmd = common::get_test_bin_command("cachegrind-reqs-test");
    cmd.assert().code(0).stdout("").stderr("");
}

#[test]
fn test_cachegrind_reqs_when_running_on_valgrind() {
    let mut cmd = common::get_valgrind_wrapper_command();
    cmd.args([
        "1",
        "--tool=cachegrind",
        "--valgrind-args=--verbose --instr-at-start=no",
        &format!(
            "--bin={}",
            common::get_test_bin_path("cachegrind-reqs-test").display()
        ),
    ]);
    let expected_code = 1;

    match cmd.assert().try_code(expected_code) {
        Ok(assert) => {
            let fixture_string = common::get_fixture_as_string("cachegrind-reqs-test.stderr");
            assert
                .stdout("")
                .stderr(predicates::str::diff(fixture_string));
        }
        Err(error) => {
            let assert = error.assert();
            let output = assert.get_output();

            let mut err = stderr();
            writeln!(err, "Unexpected exit code: STDERR:").unwrap();
            err.write_all(&output.stderr).unwrap();
            panic!(
                "Assertion of exit code failed: Actual: {}, Expected: {}",
                &output.status.code().unwrap(),
                expected_code
            )
        }
    }
}
