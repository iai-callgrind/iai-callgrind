use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, Bench, BinaryBenchmark, BinaryBenchmarkConfig,
    BinaryBenchmarkGroup, Command, Sandbox,
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
        .sandbox(Sandbox::new(true)))
]
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

fn setup_group(group: &mut BinaryBenchmarkGroup) {
    group.bench(
        BinaryBenchmark::new("some id")
            .bench(
                Bench::new("other id")
                    .command(Command::new("/usr/bin/echo").arg("1"))
                    .setup(|| {
                        println!("IN SETUP");
                        my_mod::setup_me("set me up");
                    })
                    .teardown(|| {
                        println!("IN TEARDOWN");
                        teardown(10);
                    })
                    .clone(),
            )
            .clone(),
    );
}

binary_benchmark_group!(
    name = low_level_group;
    benchmarks = |group: &mut BinaryBenchmarkGroup| setup_group(group)
);

main!(binary_benchmark_groups = low_level_group);
