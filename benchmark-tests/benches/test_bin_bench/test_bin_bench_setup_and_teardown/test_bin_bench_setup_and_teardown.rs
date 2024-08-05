use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, Bench, BinaryBenchmark, BinaryBenchmarkGroup,
};

const ECHO: &str = env!("CARGO_BIN_EXE_echo");

fn setup_no_argument() {
    println!("SETUP: setup_no_argument function");
}

fn teardown_no_argument() {
    println!("TEARDOWN: teardown_no_argument function");
}

fn setup_one_argument(arg: u64) {
    println!("SETUP: setup_one_argument function: {arg}");
}

fn teardown_one_argument(arg: u64) {
    println!("TEARDOWN: teardown_one_argument function: {arg}");
}

mod setup {
    pub fn setup_in_module(arg: u64) {
        println!("SETUP: setup::setup_in_module function: {arg}");
    }
}

mod teardown {
    pub fn teardown_in_module(arg: u64) {
        println!("TEARDOWN: teardown::teardown_in_module function: {arg}");
    }
}

#[binary_benchmark(setup = setup_no_argument(), teardown = teardown_no_argument())]
fn bench_just_binary_benchmark_attribute() -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO).arg(1.to_string()).build()
}

#[binary_benchmark]
#[bench::setup_with_args_parameter(args = (), setup = setup_one_argument(1))]
#[bench::setup_no_args_parameter(setup = setup_one_argument(2))]
fn simple_bench() -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO).arg(2.to_string()).build()
}

#[binary_benchmark]
#[bench::setup_with_one_argument(args = (3), setup = setup_one_argument(1))]
#[bench::setup_first_then_args(setup = setup_one_argument(2), args = (6))]
#[bench::setup_in_module(setup = setup::setup_in_module(3), args = (24))]
fn bench_only_setup(value: u64) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO)
        .arg(value.to_string())
        .build()
}

#[binary_benchmark]
#[bench::teardown_with_one_argument(args = (2, 3, 5), teardown = teardown_one_argument(1))]
#[bench::teardown_first_then_args(teardown = teardown_one_argument(2), args = (4, 6, 10))]
#[bench::teardown_in_module(teardown = teardown::teardown_in_module(2), args = (8, 12, 20))]
fn bench_only_teardown(a: u64, b: u64, c: u64) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO)
        .arg(a.to_string())
        .arg(b.to_string())
        .arg(c.to_string())
        .build()
}

#[binary_benchmark]
#[bench::setup_first_then_teardown(
    args = (2),
    setup = setup_one_argument(1),
    teardown = teardown_one_argument(1))
]
#[bench::teardown_first_then_setup(
    args = (3),
    teardown = teardown_one_argument(2),
    setup = setup_one_argument(2))
]
fn bench_setup_and_teardown(a: u64) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO).arg(a.to_string()).build()
}

#[binary_benchmark(setup = setup_no_argument(), teardown = teardown_no_argument())]
#[bench::no_overwrite(1)]
#[bench::overwrite_teardown(
    args = (2),
    teardown = teardown_one_argument(1)
)]
#[bench::overwrite_setup(
    args = (3),
    setup = setup_one_argument(2)
)]
#[bench::overwrite_setup_and_teardown(
    args = (4),
    setup = setup_one_argument(2),
    teardown = teardown_one_argument(1)
)]
fn bench_global_setup_and_teardown(a: u64) -> iai_callgrind::Command {
    iai_callgrind::Command::new(ECHO).arg(a.to_string()).build()
}

binary_benchmark_group!(
    name = bench_group;
    benchmarks =
        bench_just_binary_benchmark_attribute,
        simple_bench,
        bench_only_setup,
        bench_only_teardown,
        bench_setup_and_teardown,
        bench_global_setup_and_teardown
);

fn setup_low_level_group(group: &mut BinaryBenchmarkGroup) {
    group
        .binary_benchmark(
            BinaryBenchmark::new("bench_setup_last")
                .bench(Bench::new("no_setup").command(iai_callgrind::Command::new(ECHO).arg("3")))
                .bench(
                    Bench::new("with_setup")
                        .setup(|| setup_one_argument(2))
                        .command(iai_callgrind::Command::new(ECHO).arg("6")),
                ),
        )
        .binary_benchmark(
            BinaryBenchmark::new("bench_only_setup")
                .bench(
                    Bench::new("setup_with_one_argument")
                        .setup(|| setup_one_argument(1))
                        .command(iai_callgrind::Command::new(ECHO).arg("3")),
                )
                .bench(
                    Bench::new("setup_in_module")
                        .setup(|| setup::setup_in_module(2))
                        .command(iai_callgrind::Command::new(ECHO).arg("6")),
                ),
        )
        .binary_benchmark(
            BinaryBenchmark::new("bench_only_teardown")
                .bench(
                    Bench::new("teardown_with_one_argument")
                        .teardown(|| teardown_one_argument(3))
                        .command(iai_callgrind::Command::new(ECHO).arg("10")),
                )
                .bench(
                    Bench::new("teardown_in_module")
                        .teardown(|| teardown::teardown_in_module(4))
                        .command(iai_callgrind::Command::new(ECHO).arg("40")),
                ),
        )
        .binary_benchmark(
            BinaryBenchmark::new("bench_setup_and_teardown")
                .bench(
                    Bench::new("setup_first_then_teardown")
                        .setup(|| setup_one_argument(5))
                        .teardown(|| teardown_one_argument(6))
                        .command(iai_callgrind::Command::new(ECHO).arg("2")),
                )
                .bench(
                    Bench::new("teardown_first_then_setup")
                        .teardown(|| teardown_one_argument(7))
                        .setup(|| setup_one_argument(8))
                        .command(iai_callgrind::Command::new(ECHO).arg("3")),
                ),
        )
        .binary_benchmark(
            BinaryBenchmark::new("bench_global_setup_and_teardown")
                .setup(setup_no_argument)
                .teardown(teardown_no_argument)
                .bench(
                    Bench::new("no_overwrite").command(iai_callgrind::Command::new(ECHO).arg("1")),
                )
                .bench(
                    Bench::new("overwrite_teardown")
                        .teardown(|| teardown_one_argument(1))
                        .command(iai_callgrind::Command::new(ECHO).arg("2")),
                )
                .bench(
                    Bench::new("overwrite_setup")
                        .setup(|| setup_one_argument(2))
                        .command(iai_callgrind::Command::new(ECHO).arg("3")),
                )
                .bench(
                    Bench::new("overwrite_setup_and_teardown")
                        .setup(|| setup_one_argument(3))
                        .teardown(|| teardown_one_argument(4))
                        .command(iai_callgrind::Command::new(ECHO).arg("4")),
                ),
        );
}

binary_benchmark_group!(
    name = low_level;
    benchmarks = |group: &mut BinaryBenchmarkGroup| setup_low_level_group(group)
);

main!(binary_benchmark_groups = bench_group, low_level);
