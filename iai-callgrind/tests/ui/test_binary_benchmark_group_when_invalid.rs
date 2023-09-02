mod test_binary_benchmark_group_when_no_name {
    use iai_callgrind::binary_benchmark_group;
    binary_benchmark_group!(benchmark = |_group: &mut BinaryBenchmarkGroup| {});
}

mod test_binary_benchmark_group_when_no_benchmark {
    use iai_callgrind::binary_benchmark_group;
    binary_benchmark_group!(
        name = some;
        benchmark =
    );
}

fn main() {}
