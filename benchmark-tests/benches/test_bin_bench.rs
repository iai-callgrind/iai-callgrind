use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

#[binary_benchmark]
fn my_bench() -> iai_callgrind::Command {
    iai_callgrind::Command::new().arg("me").build()
}

#[binary_benchmark]
fn my_other_bench() -> iai_callgrind::Command {
    iai_callgrind::Command::new().arg("happy").build()
}

#[binary_benchmark]
#[bench::some(1)]
fn my_bench_bench(first: usize) -> iai_callgrind::Command {
    iai_callgrind::Command::new().arg(first.to_string()).build()
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
    benchmarks = my_bench, my_other_bench, my_bench_bench
);

main!(binary_benchmark_groups = my_group);
