use iai_callgrind::{main, ExitWith, Options};

main!(
    run = cmd = "benchmark-tests-exit",
    opts = Options::default().exit_with(ExitWith::Success),
    id = "succeed", args = ["0"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Failure),
          id = "fail_with_1", args = ["1"],
          id = "fail_with_2", args = ["2"],
          id = "fail_with_3", args = ["3"],
          id = "fail_with_255", args = ["255"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(0)),
          id = "code_0", args = ["0"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(1)),
          id = "code_1", args = ["1"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(2)),
          id = "code_2", args = ["2"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(3)),
          id = "code_3", args = ["3"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(255)),
          id = "code_255", args = ["-1"];
);
