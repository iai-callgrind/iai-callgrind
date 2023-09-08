mod test_main_when_library_benchmark_as_group {
    use iai_callgrind::{library_benchmark, main};
    #[library_benchmark]
    fn some_func() {}

    main!(library_benchmark_groups = some_func);
}

mod test_main_when_invalid_config {
    use iai_callgrind::{library_benchmark, library_benchmark_group, main};
    #[library_benchmark]
    fn some_func() {}

    library_benchmark_group!(
        name = my_group;
        benchmarks = some_func
    );

    main!(
        config = "some";
        library_benchmark_groups = my_group
    );
}

fn main() {}
