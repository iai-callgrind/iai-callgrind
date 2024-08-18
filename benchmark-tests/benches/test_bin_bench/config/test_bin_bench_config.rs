use std::process::Command;

use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Stdio, Tool,
    ValgrindTool,
};

#[binary_benchmark(config = BinaryBenchmarkConfig::default().env("BINARY_BENCHMARK_ENV", "3"))]
#[bench::with_env(config = BinaryBenchmarkConfig::default().env("BENCH_ENV", "4"))]
fn bench_binary() -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_env"))
        .args([
            "--check",
            "MAIN_ENV=1",
            "GROUP_ENV=2",
            "BINARY_BENCHMARK_ENV=3",
            "BENCH_ENV=4",
            "COMMAND_ENV=5",
        ])
        .env("COMMAND_ENV", "5")
        .stdout(Stdio::Inherit)
        .build()
}

fn check_setup_is_not_cleared() {
    println!("SETUP:");
    Command::new(env!("CARGO_BIN_EXE_env"))
        .args(["--is-cleared=false"])
        .status()
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_env"))
        .args([
            "--check",
            "MAIN_ENV=1",
            "GROUP_ENV=2",
            "BINARY_BENCHMARK_ENV=3",
            "BENCH_ENV=4",
            "COMMAND_ENV=5",
        ])
        .status()
        .unwrap();
}

fn check_teardown_is_not_cleared() {
    println!("TEARDOWN:");
    Command::new(env!("CARGO_BIN_EXE_env"))
        .args(["--is-cleared=false"])
        .status()
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_env"))
        .args([
            "--check",
            "MAIN_ENV=1",
            "GROUP_ENV=2",
            "BINARY_BENCHMARK_ENV=3",
            "BENCH_ENV=4",
            "COMMAND_ENV=5",
        ])
        .status()
        .unwrap();
}

#[binary_benchmark(config = BinaryBenchmarkConfig::default().env("BINARY_BENCHMARK_ENV", "3"))]
#[bench::with_env(
    setup = check_setup_is_not_cleared,
    teardown = check_teardown_is_not_cleared,
    config = BinaryBenchmarkConfig::default()
        .env("BENCH_ENV", "4")
        .tool(Tool::new(ValgrindTool::DHAT))
)]
fn check_env_is_cleared() -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_env"))
        .args(["--is-cleared=true"])
        .env("COMMAND_ENV", "5")
        .stdout(Stdio::Inherit)
        .build()
}

binary_benchmark_group!(
    name = my_group;
    config = BinaryBenchmarkConfig::default().env("GROUP_ENV", "2");
    benchmarks = bench_binary, check_env_is_cleared
);

main!(
    config = BinaryBenchmarkConfig::default().env("MAIN_ENV", "1");
    binary_benchmark_groups = my_group
);
