use std::sync::Mutex;

use iai_callgrind::{
    binary_benchmark, binary_benchmark_attribute, Bench, BenchmarkId, BinaryBenchmarkConfig,
};

static CURRENT: Mutex<String> = Mutex::new(String::new());

#[binary_benchmark(
    config = BinaryBenchmarkConfig::default().env("BINARY_BENCHMARK_ATTRIBUTE_ENV", "0")
)]
#[bench::case_1(
    args = ("1"),
    config = BinaryBenchmarkConfig::default().env("BENCH_IN_ATTRIBUTE_ENV", "1")
)]
#[bench::case_2(
    args = ("2"),
    config = BinaryBenchmarkConfig::default().env("BENCH_IN_ATTRIBUTE_ENV", "2")
)]
fn multiple_bench_with_config(id: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new("/just_testing")
        .arg("some argument")
        .env("SOME_ENV", id)
        .build()
}

fn my_setup() {
    let mut lock = CURRENT.lock().unwrap();
    "my_setup".clone_into(&mut lock);
}
fn my_teardown() {
    let mut lock = CURRENT.lock().unwrap();
    "my_teardown".clone_into(&mut lock);
}
fn my_setup_overwrite() {
    let mut lock = CURRENT.lock().unwrap();
    "my_setup_overwrite".clone_into(&mut lock);
}
fn my_teardown_overwrite() {
    let mut lock = CURRENT.lock().unwrap();
    "my_teardown_overwrite".clone_into(&mut lock);
}

#[binary_benchmark(setup = my_setup(), teardown = my_teardown())]
fn with_setup_and_teardown() -> iai_callgrind::Command {
    iai_callgrind::Command::new("/just_testing")
}

#[binary_benchmark(setup = my_setup(), teardown = my_teardown())]
#[bench::overwrite_setup(setup = my_setup_overwrite())]
#[bench::overwrite_teardown(teardown = my_teardown_overwrite())]
#[bench::overwrite_setup_and_teardown(
    setup = my_setup_overwrite(),
    teardown = my_teardown_overwrite()
)]
fn with_setup_and_teardown_overwrite() -> iai_callgrind::Command {
    iai_callgrind::Command::new("/just_testing")
}

#[test]
fn test_multiple_bench_with_config() {
    let benchmark = binary_benchmark_attribute!(multiple_bench_with_config);
    assert_eq!(benchmark.id, BenchmarkId::new("multiple_bench_with_config"));
    assert!(benchmark.teardown.is_none());
    assert!(benchmark.setup.is_none());
    assert_eq!(
        benchmark.config,
        Some(
            BinaryBenchmarkConfig::default()
                .env("BINARY_BENCHMARK_ATTRIBUTE_ENV", "0")
                .into()
        )
    );

    assert_eq!(benchmark.benches.len(), 2);

    assert_eq!(
        benchmark.benches.first().unwrap(),
        &*Bench::new("case_1")
            .config(BinaryBenchmarkConfig::default().env("BENCH_IN_ATTRIBUTE_ENV", "1"))
            .command(
                iai_callgrind::Command::new("/just_testing")
                    .arg("some argument")
                    .env("SOME_ENV", "1")
                    .build()
            )
    );

    assert_eq!(
        &benchmark.benches[1],
        &*Bench::new("case_2")
            .config(BinaryBenchmarkConfig::default().env("BENCH_IN_ATTRIBUTE_ENV", "2"))
            .command(
                iai_callgrind::Command::new("/just_testing")
                    .arg("some argument")
                    .env("SOME_ENV", "2")
                    .build()
            )
    );
}

#[test]
fn test_with_setup_and_teardown() {
    let benchmark = binary_benchmark_attribute!(with_setup_and_teardown);
    assert_eq!(benchmark.id, BenchmarkId::new("with_setup_and_teardown"));
    // This is correct, since the `#[binary_benchmark]` macro already substitutes the local setup
    // and teardown if present with the global one
    assert!(benchmark.teardown.is_none());
    assert!(benchmark.setup.is_none());
    assert_eq!(benchmark.config, None);

    assert_eq!(benchmark.benches.len(), 1);

    let bench = benchmark.benches.first().unwrap();
    assert_eq!(
        bench,
        &*Bench::new("with_setup_and_teardown")
            .setup(bench.setup.unwrap())
            .teardown(bench.teardown.unwrap())
            .command(iai_callgrind::Command::new("/just_testing"))
    );
}

#[test]
// To make the accesses to CURRENT safe we run this test serially
#[serial_test::serial]
fn test_with_setup_and_teardown_overwrite() {
    let benchmark = binary_benchmark_attribute!(with_setup_and_teardown_overwrite);
    assert_eq!(
        benchmark.id,
        BenchmarkId::new("with_setup_and_teardown_overwrite")
    );
    assert!(benchmark.teardown.is_none());
    assert!(benchmark.setup.is_none());
    assert_eq!(benchmark.config, None);

    assert_eq!(benchmark.benches.len(), 3);

    let bench = benchmark.benches.first().unwrap();
    bench.setup.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_setup_overwrite");
    bench.teardown.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_teardown");

    assert_eq!(
        bench,
        &*Bench::new("overwrite_setup")
            .setup(bench.setup.unwrap())
            .teardown(bench.teardown.unwrap())
            .command(iai_callgrind::Command::new("/just_testing"))
    );

    let bench = &benchmark.benches[1];
    bench.setup.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_setup");
    bench.teardown.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_teardown_overwrite");

    assert_eq!(
        bench,
        &*Bench::new("overwrite_teardown")
            .setup(bench.setup.unwrap())
            .teardown(bench.teardown.unwrap())
            .command(iai_callgrind::Command::new("/just_testing"))
    );

    let bench = &benchmark.benches[2];
    bench.setup.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_setup_overwrite");
    bench.teardown.unwrap()();
    assert_eq!(CURRENT.lock().unwrap().as_str(), "my_teardown_overwrite");

    assert_eq!(
        bench,
        &*Bench::new("overwrite_setup_and_teardown")
            .setup(bench.setup.unwrap())
            .teardown(bench.teardown.unwrap())
            .command(iai_callgrind::Command::new("/just_testing"))
    );
}
