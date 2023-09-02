mod test_main_when_no_config_no_function {
    use iai_callgrind::main;
    fn some_func() {}
    main!(functions = );
}

mod test_main_when_wrong_config {
    use iai_callgrind::{main, BinaryBenchmarkConfig};
    fn some_func() {}
    main!(
        config = BinaryBenchmarkConfig::default();
        functions = some_func);
}

mod test_main_when_literal_as_config {
    use iai_callgrind::{main, BinaryBenchmarkConfig};
    fn some_func() {}
    main!(
        config = "my_config";
        functions = some_func);
}

fn main() {}
