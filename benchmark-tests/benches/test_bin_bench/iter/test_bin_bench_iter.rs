use iai_callgrind::{
    binary_benchmark, binary_benchmark_attribute, binary_benchmark_group, main, Bench,
    BinaryBenchmark, BinaryBenchmarkConfig, Dhat,
};

const ECHO: &str = env!("CARGO_BIN_EXE_echo");

#[binary_benchmark]
#[benches::two(iter = vec![(1, 2), (3, 4)])]
fn bench_tuple((first, second): (u64, u64)) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO)
        .arg(first.to_string())
        .arg(second.to_string())
        .build()
}

pub fn num_to_string(num: u64) -> String {
    num.to_string()
}

#[binary_benchmark(
    config = BinaryBenchmarkConfig::default()
        .tool(Dhat::default().frames(["*::num_to_string"]))
)]
#[benches::single(iter = vec![1, 2])]
#[benches::with_setup_path(iter = vec![1, 2], setup = num_to_string)]
#[benches::with_setup(iter = vec![1, 2], setup = num_to_string(10))]
#[benches::with_teardown_path(iter = vec![1, 2], teardown = num_to_string)]
#[benches::with_teardown(iter = vec![1, 2], teardown = num_to_string(20))]
#[benches::with_setup_and_teardown_both_path(
    iter = vec![1, 2],
    setup = num_to_string,
    teardown = num_to_string
)]
#[benches::with_setup_path_and_teardown(
    iter = vec![1, 2],
    setup = num_to_string,
    teardown = num_to_string(30)
)]
#[benches::with_setup_and_teardown_path(
    iter = vec![1, 2],
    setup = num_to_string(10),
    teardown = num_to_string
)]
#[benches::with_setup_and_teardown(
    iter = vec![1, 2],
    setup = num_to_string(10),
    teardown = num_to_string(20)
)]
#[benches::range(iter = 1..=5)]
#[benches::iterator(iter = vec![1, 2].into_iter().map(|n| n + 10))]
fn bench(num: u64) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO)
        .arg(num.to_string())
        .build()
}

binary_benchmark_group!(
    name = high_level;
    benchmarks = bench_tuple, bench
);

binary_benchmark_group!(
    name = low_level;
    compare_by_id = true;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            .binary_benchmark(
                binary_benchmark_attribute!(bench)
            )
            .binary_benchmark(
                BinaryBenchmark::new("low_level_benchmark")
                    .bench(
                        Bench::new("foo")
                            .command(iai_callgrind::Command::new(ECHO))
                    )
            )
            .binary_benchmark(
                BinaryBenchmark::new("low_level_other")
                    .bench(
                        Bench::new("foo")
                            .command(iai_callgrind::Command::new(ECHO).arg("foo"))
                    )
                )
    }
);

main!(binary_benchmark_groups = low_level);
