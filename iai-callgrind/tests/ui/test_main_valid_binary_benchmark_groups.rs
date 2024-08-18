mod test_main_when_single_group {
    use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

    #[binary_benchmark]
    fn some_bench() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some_path")
    }

    binary_benchmark_group!(
        name = some;
        benchmarks = some_bench
    );

    main!(binary_benchmark_groups = some);
}

mod test_main_when_multiple_groups {
    use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

    #[binary_benchmark]
    fn some_bench() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some_path")
    }

    binary_benchmark_group!(
        name = some;
        benchmarks = some_bench
    );

    binary_benchmark_group!(
        name = some_other;
        benchmarks = some_bench
    );

    main!(binary_benchmark_groups = some, some_other);
}

fn main() {}
