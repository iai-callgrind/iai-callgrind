mod test_no_callgrind_args {
    use iai_callgrind::main;
    fn some_func() {}
    main!(callgrind_args = ; functions = some_func);
}

mod test_one_callgrind_args {
    use iai_callgrind::main;
    fn some_func() {}
    main!(callgrind_args = "some"; functions = some_func);
}

mod test_two_callgrind_args {
    use iai_callgrind::main;
    fn some_func() {}
    main!(callgrind_args = "some", "other"; functions = some_func);
}

mod test_multiple_callgrind_args {
    use iai_callgrind::main;
    fn some_func() {}
    main!(callgrind_args = "0", "1", "2", "3", "4", "5", "6"; functions = some_func);
}

mod test_multiple_functions {
    use iai_callgrind::main;
    fn some_func() {}
    fn some_other_func() {}
    main!(callgrind_args = "0", "1"; functions = some_func, some_other_func);
}

fn main() {}
