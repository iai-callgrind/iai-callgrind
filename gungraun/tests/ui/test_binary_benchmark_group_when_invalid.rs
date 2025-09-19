mod test_binary_benchmark_group_when_no_name_1 {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(
        benchmarks = |_group| {
            _group;
        }
    );
}

mod test_binary_benchmark_group_when_no_name_2 {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(benchmarks = |_group: &mut BinaryBenchmarkGroup| {});
}

mod test_binary_benchmark_group_when_no_name_3 {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(benchmarks = |_group| {});
}

mod test_binary_benchmark_group_when_no_benchmark_argument {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(
        name = some;
    );
}

mod test_binary_benchmark_group_when_no_benchmark {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(
        name = some;
        benchmarks =
    );
}

mod test_binary_benchmark_group_low_level_when_no_benchmark_1 {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(
        name = some;
        benchmarks = |group|
    );
}

mod test_binary_benchmark_group_low_level_when_no_benchmark_2 {
    use gungraun::binary_benchmark_group;
    binary_benchmark_group!(
        name = some;
        benchmarks = |group: &mut BinaryBenchmarkGroup|
    );
}

fn main() {}
