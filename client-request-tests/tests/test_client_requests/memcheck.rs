use crate::common;

#[test]
fn test_memcheck_reqs_when_running_native() {
    let mut cmd = common::get_test_bin_command("memcheck-reqs-test");
    cmd.assert().code(0).stdout("").stderr("");
}

#[test]
fn test_memcheck_reqs_when_running_on_valgrind() {
    let mut cmd = common::get_valgrind_wrapper_command();
    cmd.args([
        "1",
        "--tool=memcheck",
        "--valgrind-args=--verbose",
        &format!(
            "--bin={}",
            common::get_test_bin_path("memcheck-reqs-test").display()
        ),
    ]);
    cmd.assert()
        .code(1)
        .stdout("")
        .stderr(predicates::str::diff(common::get_fixture_as_string(
            "memcheck-reqs-test.stderr",
        )));
}
