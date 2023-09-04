mod test_main_callgrind_args_when_integer_literal {
    use iai_callgrind::main;
    fn some_func() {}
    main!(callgrind_args = 1; functions = some_func);
}

mod test_main_when_function_is_a_module {
    use iai_callgrind::main;
    mod my_mod {}
    main!(callgrind_args = ; functions = my_mod);
}

mod test_main_when_function_is_a_string {
    use iai_callgrind::main;
    main!(callgrind_args = ; functions = "some_func");
}

mod test_main_when_function_is_within_a_module {
    use iai_callgrind::main;
    mod my_mod {
        fn some_func() {}
    }
    main!(callgrind_args = ; functions = my_mod::some_func);
}

fn main() {}
