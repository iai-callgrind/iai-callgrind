mod test_main_when_no_config {
    use iai_callgrind::main;
    fn some_func() {}
    main!(functions = some_func);
}

mod test_main_when_two_functions {
    use iai_callgrind::main;
    fn func1() {}
    fn func2() {}
    main!(functions = func1, func2);
}

mod test_main_when_multiple_functions {
    use iai_callgrind::main;
    fn func1() {}
    fn func2() {}
    fn func3() {}
    fn func4() {}
    fn func5() {}
    main!(functions = func1, func2, func3, func4, func5);
}

mod test_main_when_config_default {
    use iai_callgrind::{main, LibraryBenchmarkConfig};
    fn some_func() {}
    main!(
        config = LibraryBenchmarkConfig::default();
        functions = some_func);
}

mod test_main_when_config_ref {
    use iai_callgrind::{main, LibraryBenchmarkConfig};
    fn some_func() {}
    main!(
        config = &LibraryBenchmarkConfig::default();
        functions = some_func);
}

mod test_main_when_config_mut_ref {
    use iai_callgrind::{main, LibraryBenchmarkConfig};
    fn some_func() {}
    main!(
        config = LibraryBenchmarkConfig::default().raw_callgrind_args(["hello=world"]);
        functions = some_func);
}

fn main() {}
