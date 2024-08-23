use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

#[binary_benchmark]
#[bench::some_id("foo.txt")]
fn bench_binary(path: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_echo"))
        .arg(path)
        .build()
}

binary_benchmark_group!(
    name = my_group;
    benchmarks = bench_binary
);

main!(binary_benchmark_groups = my_group);
