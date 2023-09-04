mod test_one_function {
    use iai_callgrind::main;
    fn some_func() {}
    main!(some_func);
}

mod test_two_functions {
    use iai_callgrind::main;
    fn some_func() {}
    fn some_other_func() {}
    main!(some_func, some_other_func);
}

mod test_multiple_functions {
    use iai_callgrind::main;
    fn some1() {}
    fn some2() {}
    fn some3() {}
    fn some4() {}
    fn some5() {}
    main!(some1, some2, some3, some4, some5);
}

fn main() {}
