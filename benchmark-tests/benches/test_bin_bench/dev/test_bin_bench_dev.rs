use iai_callgrind::{
    binary_benchmark, binary_benchmark_attribute, binary_benchmark_group, main, Bench,
    BinaryBenchmark, BinaryBenchmarkConfig, BinaryBenchmarkGroup, Command, Sandbox,
};

#[binary_benchmark]
fn just_outer_attribute() -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg("me")
        .build()
}

#[binary_benchmark(config = BinaryBenchmarkConfig::default())]
fn bench_with_config() -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg("happy")
        .build()
}

#[binary_benchmark]
#[bench::some(1)]
fn bench(first: usize) -> iai_callgrind::Command {
    // iai_callgrind::Command::new("/usr/bin/echo")
    //     .arg(first.to_string())
    //     .build()
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg(first.to_string())
        .build()
}

#[binary_benchmark]
#[benches::multiple_list(1, 2, 3)]
#[benches::multiple_args(
    args = [1, 2, 3],
    setup = my_mod::setup_me("hello there"),
    config = BinaryBenchmarkConfig::default()
        .sandbox(Sandbox::new(true))
)]
fn benches(first: usize) -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg(first.to_string())
        .build()
}

mod my_mod {
    pub fn setup_me<T>(arg: T)
    where
        T: AsRef<str>,
    {
        println!("{}", arg.as_ref());
    }
}

fn setup(size: usize) {
    println!("setup: {size}");
}

fn teardown(size: usize) {
    println!("teardown: {size}");
}

binary_benchmark_group!(
    name = my_group;
    setup = setup(10);
    teardown = teardown(20);
    benchmarks = just_outer_attribute, bench_with_config, bench, benches
);

// TODO: Test invalid benchmark ids
fn setup_group(group: &mut BinaryBenchmarkGroup) {
    group
        .binary_benchmark(
            BinaryBenchmark::new("some_id")
                .setup(|| println!("IN BINARY BENCHMARK SETUP"))
                .teardown(|| println!("IN BINARY BENCHMARK TEARDOWN"))
                .bench(
                    Bench::new("other_id")
                        .config(BinaryBenchmarkConfig::default())
                        .command(Command::new("/usr/bin/echo").arg("1"))
                        .setup(|| {
                            println!("IN SETUP");
                            my_mod::setup_me("set me up");
                        })
                        .teardown(|| {
                            println!("IN TEARDOWN");
                            teardown(10);
                        }),
                )
                .bench(
                    Bench::new("global_setup_and_teardown")
                        .command(Command::new("/usr/bin/echo").arg("2")),
                ),
        )
        .binary_benchmark(binary_benchmark_attribute!(benches));
}

binary_benchmark_group!(
    name = low_level_group;
    benchmarks = |group: &mut BinaryBenchmarkGroup| setup_group(group)
);

#[binary_benchmark]
#[bench::some_id("foo")]
fn benchmark_echo(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = some_group;
    benchmarks = benchmark_echo
);

binary_benchmark_group!(
    name = low_level_old;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            .binary_benchmark(
                BinaryBenchmark::new("benchmark_echo")
                    .bench(
                        Bench::new("some_id").command(
                            iai_callgrind::Command::new("/usr/bin/echo").arg("foo")
                        )
                    )
            )
    }
);

#[binary_benchmark]
#[bench::some_id("foo")]
fn attribute_benchmark_echo(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = low_level;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            // Add a benchmark function with the #[binary_benchmark]
            // attribute with the `binary_benchmark_attribute!` macro
            .binary_benchmark(binary_benchmark_attribute!(attribute_benchmark_echo))
            // For the sake of simplicity, assume this would be the benchmark you
            // were not able to setup with the attribute
            .binary_benchmark(
                BinaryBenchmark::new("low_level_benchmark_echo")
                    .bench(
                        Bench::new("some_id").command(
                            iai_callgrind::Command::new("/usr/bin/echo").arg("foo")
                        )
                    )
            )
    }
);

fn my_setup(arg: &str) {
    println!("SEtUP: {arg}");
}

#[binary_benchmark]
#[bench::some(args = ("1"), setup = my_setup)]
fn benchmark_setup(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new("/usr/bin/echo")
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = bench_with_setup;
    benchmarks = benchmark_setup
);

main!(
    binary_benchmark_groups = low_level_group,
    my_group,
    some_group,
    low_level,
    bench_with_setup
);
