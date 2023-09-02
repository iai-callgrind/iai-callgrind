mod test_main_when_binary_benchmark_group_is_not_a_group {
    use iai_callgrind::main;

    fn some_func() {}

    main!(binary_benchmark_groups = some_func);
}

mod test_main_when_config_is_not_a_binary_benchmark_config {
    use iai_callgrind::{binary_benchmark_group, main};

    binary_benchmark_group!(
        name = some;
        benchmark = |group: &mut BinaryBenchmarkGroup| {
            // do nothing
        }
    );

    main!(
        config = "some string";
        binary_benchmark_groups = some
    );
}

mod test_main_when_no_group {
    use iai_callgrind::main;
    main!(binary_benchmark_groups = );
}

fn main() {}
