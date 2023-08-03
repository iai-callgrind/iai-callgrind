use iai_callgrind::main;

main!(
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Success),
          args = ["0"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Failure),
          args = ["1"], args = ["2"], args = ["3"], args = ["255"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(0)),
          args = ["0"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(1)),
          args = ["1"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(2)),
          args = ["2"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(3)),
          args = ["3"];
    run = cmd = "benchmark-tests-exit",
          opts = Options::default().exit_with(ExitWith::Code(255)),
          args = ["-1"];
);
