mod test_setup_as_path_too_few_arguments {
    use iai_callgrind::binary_benchmark;

    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(setup = setup)]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_setup_as_path_too_many_arguments {
    use iai_callgrind::binary_benchmark;

    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(args = ("1", "2"), setup = setup)]
    fn bench_binary(a: &str, b: &str) -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
            .args([a.to_string(), b.to_string()])
            .build()
    }
}

mod test_teardown_as_path_too_few_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(teardown = teardown)]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_teardown_as_path_too_many_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(args = ("1", "2"), teardown = teardown)]
    fn bench_binary(a: &str, b: &str) -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
            .args([a.to_string(), b.to_string()])
            .build()
    }
}

mod test_setup_too_few_arguments {
    use iai_callgrind::binary_benchmark;

    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(setup = setup())]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_setup_too_many_arguments {
    use iai_callgrind::binary_benchmark;

    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(setup = setup("1", "2"))]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_teardown_too_few_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(teardown = teardown())]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_teardown_too_many_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(teardown = teardown("1", "2"))]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_setup_too_many_teardown_too_many_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}
    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(setup = setup("1", "2"), teardown = teardown("1", "2"))]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

mod test_setup_as_path_too_few_teardown_too_few_arguments {
    use iai_callgrind::binary_benchmark;

    fn teardown(_arg: &str) {}
    fn setup(_arg: &str) {}

    #[binary_benchmark]
    #[bench::some(setup = setup, teardown = teardown)]
    fn bench_binary() -> iai_callgrind::Command {
        iai_callgrind::Command::new("some")
    }
}

fn main() {}
