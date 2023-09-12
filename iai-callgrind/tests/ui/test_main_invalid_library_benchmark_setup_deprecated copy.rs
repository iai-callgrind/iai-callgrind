mod test_main_deprecated {
    use iai_callgrind::main;
    fn some_func() {}
    main!(run = cmd = "some", id = "id", args = []);
}

fn main() {}
