use std::path::PathBuf;

use gungraun::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, OutputFormat, Sandbox,
};

const FILE_EXISTS: &str = env!("CARGO_BIN_EXE_file-exists");

fn check_file_exists(path: &str, should_exist: bool) {
    if should_exist {
        assert!(PathBuf::from(path).is_file());
        println!("File exists: '{path}'")
    } else {
        assert!(!PathBuf::from(path).exists());
        println!("File does not exist: '{path}'")
    }
}

#[binary_benchmark]
#[bench::sandbox_with_fixture(
    args = ("one_line.fix", true),
    setup = check_file_exists,
    teardown = check_file_exists,
    config = BinaryBenchmarkConfig::default()
        .sandbox(Sandbox::new(true)
            .fixtures(["benchmark-tests/benches/fixtures/one_line.fix"])
        )
)]
#[bench::sandbox_without_fixture(
    args = ("one_line.fix", false),
    setup = check_file_exists,
    teardown = check_file_exists,
    config = BinaryBenchmarkConfig::default()
        .sandbox(Sandbox::new(true))
)]
fn with_sandbox(path: &str, exists: bool) -> gungraun::Command {
    gungraun::Command::new(FILE_EXISTS)
        .arg(path)
        .arg(exists.to_string())
        .build()
}

#[binary_benchmark()]
#[bench::check_file(
    args = ("benches/fixtures/one_line.fix", true),
    config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(false)),
    setup = check_file_exists,
    teardown = check_file_exists
)]
fn without_sandbox(path: &str, should_exist: bool) -> gungraun::Command {
    gungraun::Command::new(FILE_EXISTS)
        .arg(path)
        .arg(should_exist.to_string())
        .build()
}

fn setup_directory_and_file() {
    std::fs::create_dir("foo").unwrap();
    std::fs::write("foo/bar.txt", "bar").unwrap();
    println!("Created directory 'foo' with file 'bar.txt'");
}

fn teardown_directory_and_file() {
    std::fs::remove_file("foo/bar.txt").unwrap();
    std::fs::remove_dir("foo").unwrap();
    println!("Deleted directory 'foo' with file 'bar.txt'");
}

#[binary_benchmark(setup = setup_directory_and_file(), teardown = teardown_directory_and_file)]
fn with_current_dir() -> gungraun::Command {
    gungraun::Command::new(FILE_EXISTS)
        .current_dir("foo")
        .arg("bar.txt")
        .arg("true")
        .build()
}

binary_benchmark_group!(
    name = my_group;
    benchmarks = with_sandbox, without_sandbox, with_current_dir
);

main!(
    config = BinaryBenchmarkConfig::default()
        .sandbox(Sandbox::new(true))
        .output_format(OutputFormat::default()
            .truncate_description(None)
        );
    binary_benchmark_groups = my_group
);
