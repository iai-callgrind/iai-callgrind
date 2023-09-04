mod test_main_when_library_benchmark_as_group {
    use iai_callgrind::{library_benchmark, main};
    #[library_benchmark]
    fn some_func() {}

    main!(library_benchmark_groups = some_func);
}

fn main() {}
