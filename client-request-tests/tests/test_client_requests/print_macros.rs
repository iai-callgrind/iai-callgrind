use crate::common::{self};

#[test]
fn test_client_request_print_macros_when_running_native() {
    let mut cmd = common::get_test_bin_command("print-macros-test");
    cmd.assert().code(0).stdout("").stderr("");
}

#[test]
fn test_client_request_print_macros_when_running_on_valgrind() {
    let mut cmd = common::get_valgrind_wrapper_command();
    cmd.args([
        "1",
        "--tool=callgrind",
        "--valgrind-args=--verbose",
        &format!(
            "--bin={}",
            common::get_test_bin_path("print-macros-test").display()
        ),
    ]);
    let sandbox = common::get_sandbox();
    cmd.current_dir(&sandbox);
    cmd.assert()
        .code(1)
        .stdout("")
        .stderr(common::get_fixture_as_string("print-macros-test.stderr"));
    drop(sandbox);
}
