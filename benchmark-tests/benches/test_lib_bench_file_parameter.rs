use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

/// See issue https://github.com/iai-callgrind/iai-callgrind/issues/198
/// Generic bench arguments cause compilation failure
///
/// After the fix the benchmark should now compile
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

#[library_benchmark]
#[benches::one_line(file = "benchmark-tests/benches/fixtures/one_line.fix")]
fn one_line(value: String) {
    assert_eq!(value, "1".to_owned());
}

#[library_benchmark]
#[benches::two_lines(file = "benchmark-tests/benches/fixtures/two_lines.fix")]
fn two_lines(value: String) {
    let lock_file = "/tmp/iai-callgrind.two_lines.lock";
    if &value == "11" {
        if PathBuf::from_str(lock_file).unwrap().exists() {
            panic!("The lock file '{lock_file}' should not exist");
        }
        File::create(lock_file).unwrap();
    } else {
        std::fs::remove_file(lock_file).unwrap();
        assert_eq!(value, "111".to_owned());
    }
}

library_benchmark_group!(
    name = bench_group;
    benchmarks = one_line, two_lines
);

main!(library_benchmark_groups = bench_group);
