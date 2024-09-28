//! Contains macros which together define a benchmark harness that can be used in place of the
//! standard benchmark harness. This allows the user to run Iai benchmarks with `cargo bench`.

/// [low level api](`crate::binary_benchmark_group`) only: Use to add a `#[binary_benchmark]` to a
/// [`crate::BinaryBenchmarkGroup`]
///
/// # Examples
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{binary_benchmark_attribute, binary_benchmark_group, binary_benchmark};
///
/// #[binary_benchmark]
/// fn bench_binary() -> iai_callgrind::Command {
///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///         .arg("foo")
///         .build()
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
///         group.binary_benchmark(binary_benchmark_attribute!(bench_binary));
///     }
/// );
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! binary_benchmark_attribute {
    ($name:ident) => {{
        let mut binary_benchmark = $crate::BinaryBenchmark::new(stringify!($name));
        binary_benchmark.config = $name::__get_config();

        for internal_bench in $name::__BENCHES {
            let mut bench = if let Some(id) = internal_bench.id_display {
                $crate::Bench::new(id)
            } else {
                $crate::Bench::new(stringify!($name))
            };
            let mut bench = bench.command((internal_bench.func)());
            if let Some(setup) = internal_bench.setup {
                bench.setup(setup);
            }
            if let Some(teardown) = internal_bench.teardown {
                bench.teardown(teardown);
            }
            if let Some(config) = internal_bench.config {
                bench.config(config());
            }
            binary_benchmark.bench(bench);
        }
        binary_benchmark
    }};
}

/// The `iai_callgrind::main` macro expands to a `main` function which runs all the benchmarks.
///
/// Using Iai-callgrind requires disabling the benchmark harness. This can be done like so in the
/// `Cargo.toml` file:
///
/// ```toml
/// [[bench]]
/// name = "my_bench"
/// harness = false
/// ```
///
/// To be able to run any iai-callgrind benchmarks, you'll also need the `iai-callgrind-runner`
/// installed with the binary somewhere in your `$PATH` for example with
///
/// ```shell
/// cargo install iai-callgrind-runner
/// ```
///
/// `my_bench` has to be a rust file inside the 'benches' directory.
///
/// # Library Benchmarks
///
/// The [`crate::main`] macro has one form to run library benchmarks:
///
/// ```rust
/// # use iai_callgrind::{main, library_benchmark_group, library_benchmark};
/// # #[library_benchmark]
/// # fn bench_fibonacci() { }
/// # library_benchmark_group!(
/// #    name = some_group;
/// #    benchmarks = bench_fibonacci
/// # );
/// # fn main() {
/// main!(library_benchmark_groups = some_group);
/// # }
/// ```
///
/// which accepts the following top-level arguments in this order (separated by a semicolon):
///
/// * __`config`__ (optional): Optionally specify a [`crate::LibraryBenchmarkConfig`] valid for all
///   benchmark groups
/// * __`setup`__ (optional): A setup function or any valid expression which is run before all
///   benchmarks
/// * __`teardown`__ (optional): A setup function or any valid expression which is run after all
///   benchmarks
/// * __`library_benchmark_groups`__ (mandatory): The __name__ of one or more
///   [`library_benchmark_group!`](crate::library_benchmark_group) macros. Multiple __names__ are
///   expected to be a comma separated list
///
/// A library benchmark consists of
/// [`library_benchmark_groups`](crate::library_benchmark_group) and with
/// [`#[library_benchmark]`](crate::library_benchmark) annotated benchmark functions.
///
/// ```rust
/// use iai_callgrind::{main, library_benchmark_group, library_benchmark};
/// use std::hint::black_box;
///
/// fn fibonacci(n: u64) -> u64 {
///     match n {
///         0 => 1,
///         1 => 1,
///         n => fibonacci(n - 1) + fibonacci(n - 2),
///     }
/// }
///
/// #[library_benchmark]
/// #[bench::short(10)]
/// #[bench::long(30)]
/// fn bench_fibonacci(value: u64) -> u64 {
///     black_box(fibonacci(value))
/// }
///
/// library_benchmark_group!(
///     name = bench_fibonacci_group;
///     benchmarks = bench_fibonacci
/// );
///
/// # fn main() {
/// main!(library_benchmark_groups = bench_fibonacci_group);
/// # }
/// ```
///
/// If you need to pass arguments to valgrind's callgrind, you can specify raw callgrind
/// arguments via the [`crate::LibraryBenchmarkConfig`]:
///
/// ```rust
/// # use iai_callgrind::{main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig};
/// # #[library_benchmark]
/// # fn bench_fibonacci() { }
/// # library_benchmark_group!(
/// #    name = some_group;
/// #    benchmarks = bench_fibonacci
/// # );
/// # fn main() {
/// main!(
///     config = LibraryBenchmarkConfig::default()
///                 .raw_callgrind_args(
///                     ["--arg-with-flags=yes", "arg-without-flags=is_ok_too"]
///                 );
///     library_benchmark_groups = some_group
/// );
/// # }
/// ```
///
/// See also [Callgrind Command-line
/// options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options).
///
/// For an in-depth description of library benchmarks and more examples see the
/// [README#Library
/// Benchmarks](https://github.com/iai-callgrind/iai-callgrind#library-benchmarks) of this
/// crate.
///
/// # Binary Benchmarks
///
/// Setting up binary benchmarks is almost the same as setting up library benchmarks but using the
/// `#[binary_benchmark]` macro. For example, if you're crate's binary is called `my-foo`:
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{main, binary_benchmark_group, binary_benchmark};
///
/// #[binary_benchmark]
/// #[bench::hello_world("hello world")]
/// #[bench::foo("foo")]
/// fn bench_binary(arg: &str) -> iai_callgrind::Command {
///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///         .arg(arg)
///         .build()
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = bench_binary
/// );
///
/// # fn main() {
/// main!(binary_benchmark_groups = my_group);
/// # }
/// ```
///
/// See the documentation of [`crate::binary_benchmark_group`] and [`crate::Command`] for more
/// details.
#[macro_export]
macro_rules! main {
    ( $( options = $( $options:literal ),+ $(,)*; )?
      $( before = $before:ident $(, bench = $bench_before:literal )? ; )?
      $( after = $after:ident $(, bench = $bench_after:literal )? ; )?
      $( setup = $setup:ident $(, bench = $bench_setup:literal )? ; )?
      $( teardown = $teardown:ident $(, bench = $bench_teardown:literal )? ; )?
      $( sandbox = $sandbox:literal; )?
      $( fixtures = $fixtures:literal $(, follow_symlinks = $follow_symlinks:literal )? ; )?
      $( run = cmd = $cmd:expr
            $(, envs = [ $( $envs:literal ),* $(,)* ] )?,
            $( id = $id:literal, args = [ $( $args:literal ),* $(,)* ]  ),+ $(,)*
      );+ $(;)*
    ) => {
        compile_error!(
            "You are using a deprecated syntax of the main! macro to set up binary benchmarks. \
            See the README (https://github.com/iai-callgrind/iai-callgrind) and \
            docs (https://docs.rs/iai-callgrind/latest/iai_callgrind/) for further details."
        );
        pub fn main() {}
    };
    (
        $( config = $config:expr; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        binary_benchmark_groups =
    ) => {
        compile_error!("The binary_benchmark_groups argument needs at least one `name` of a `binary_benchmark_group!`");
    };
    (
        $( config = $config:expr; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        binary_benchmark_groups = $( $group:ident ),+ $(,)*
    ) => {
        fn __run() -> Result<(), $crate::error::Errors> {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.13.4";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--bin-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(env!("CARGO_PKG_NAME"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable benchmark binary

            let mut config: Option<$crate::internal::InternalBinaryBenchmarkConfig> = None;
            $(
                config = Some($config.into());
            )?

            let mut internal_benchmark_groups = $crate::internal::InternalBinaryBenchmarkGroups {
                config: config.unwrap_or_default(),
                command_line_args: this_args.collect(),
                has_setup: __run_setup(false),
                has_teardown: __run_teardown(false),
                ..Default::default()
            };

            let mut errors = $crate::error::Errors::default();

            $(
                if $group::__IS_ATTRIBUTE {
                    let mut internal_group = $crate::internal::InternalBinaryBenchmarkGroup {
                        id: stringify!($group).to_owned(),
                        config: $group::__get_config(),
                        binary_benchmarks: vec![],
                        has_setup: $group::__run_setup(false),
                        has_teardown: $group::__run_teardown(false),
                        compare_by_id: $group::__compare_by_id()
                    };
                    for (function_name, get_config, macro_bin_benches) in $group::__BENCHES {
                        let mut internal_binary_benchmark =
                            $crate::internal::InternalBinaryBenchmark {
                                benches: vec![],
                                config: get_config()
                        };
                        for macro_bin_bench in macro_bin_benches.iter() {
                            let bench = $crate::internal::InternalBinaryBenchmarkBench {
                                id: macro_bin_bench.id_display.map(|i| i.to_string()),
                                args: macro_bin_bench.args_display.map(|i| i.to_string()),
                                function_name: function_name.to_string(),
                                command: (macro_bin_bench.func)().into(),
                                config: macro_bin_bench.config.map(|f| f()),
                                has_setup: macro_bin_bench.setup.is_some(),
                                has_teardown: macro_bin_bench.teardown.is_some()
                            };
                            internal_binary_benchmark.benches.push(bench);
                        }
                        internal_group.binary_benchmarks.push(internal_binary_benchmark);
                    }

                    internal_benchmark_groups.groups.push(internal_group);
                } else {
                    let mut group = $crate::BinaryBenchmarkGroup::default();
                    $group::$group(&mut group);

                    let module_path = module_path!();

                    let mut internal_group = $crate::internal::InternalBinaryBenchmarkGroup {
                        id: stringify!($group).to_owned(),
                        config: $group::__get_config(),
                        binary_benchmarks: vec![],
                        has_setup: $group::__run_setup(false),
                        has_teardown: $group::__run_teardown(false),
                        compare_by_id: $group::__compare_by_id()
                    };

                    let mut binary_benchmark_ids =
                        std::collections::HashSet::<$crate::BenchmarkId>::new();

                    if group.binary_benchmarks.is_empty() {
                        errors.add(
                            $crate::error::Error::GroupError(
                                module_path.to_owned(),
                                internal_group.id.clone(),
                                "This group needs at least one benchmark".to_owned()
                            )
                        );
                    }

                    for binary_benchmark in group.binary_benchmarks {
                        if let Err(message) = binary_benchmark.id.validate() {
                            errors.add(
                                $crate::error::Error::BinaryBenchmarkError(
                                    module_path.to_owned(),
                                    internal_group.id.clone(),
                                    binary_benchmark.id.to_string(),
                                    message
                                )
                            );
                            continue;
                        }
                        if !binary_benchmark_ids.insert(binary_benchmark.id.clone()) {
                            errors.add(
                                $crate::error::Error::BinaryBenchmarkError(
                                    module_path.to_owned(),
                                    internal_group.id.clone(),
                                    binary_benchmark.id.to_string(),
                                    "Duplicate binary benchmark id".to_owned()
                                )
                            );
                            continue;
                        }

                        let mut internal_binary_benchmark =
                            $crate::internal::InternalBinaryBenchmark {
                                benches: vec![],
                                config: binary_benchmark.config.map(Into::into)
                        };

                        let mut bench_ids =
                            std::collections::HashSet::<$crate::BenchmarkId>::new();

                        if binary_benchmark.benches.is_empty() {
                            errors.add(
                                $crate::error::Error::BinaryBenchmarkError(
                                    module_path.to_owned(),
                                    internal_group.id.clone(),
                                    binary_benchmark.id.to_string(),
                                    "This binary benchmark needs at least one bench".to_owned()
                                )
                            );
                        }

                        for bench in binary_benchmark.benches {
                            match bench.commands.as_slice() {
                                [] => {
                                    errors.add(
                                        $crate::error::Error::BenchError(
                                            module_path.to_owned(),
                                            internal_group.id.clone(),
                                            binary_benchmark.id.to_string(),
                                            bench.id.to_string(),
                                            "Missing command".to_owned()
                                        )
                                    );
                                },
                                [command] => {
                                    if let Err(message) = bench.id.validate() {
                                        errors.add(
                                            $crate::error::Error::BenchError(
                                                module_path.to_owned(),
                                                internal_group.id.clone(),
                                                binary_benchmark.id.to_string(),
                                                bench.id.to_string(),
                                                message
                                            )
                                        );
                                    }
                                    if !bench_ids.insert(bench.id.clone()) {
                                        errors.add(
                                            $crate::error::Error::BenchError(
                                                module_path.to_owned(),
                                                internal_group.id.clone(),
                                                binary_benchmark.id.to_string(),
                                                bench.id.to_string(),
                                                format!("Duplicate id: '{}'", bench.id)
                                            )
                                        );
                                    }
                                    let internal_bench =
                                        $crate::internal::InternalBinaryBenchmarkBench {
                                            id: Some(bench.id.into()),
                                            args: None,
                                            function_name: binary_benchmark.id.clone().into(),
                                            command: command.into(),
                                            config: bench.config.clone(),
                                            has_setup: bench.setup.is_some()
                                                    || binary_benchmark.setup.is_some(),
                                            has_teardown: bench.teardown.is_some()
                                                    || binary_benchmark.teardown.is_some(),
                                    };
                                    internal_binary_benchmark.benches.push(internal_bench);
                                },
                                commands => {
                                    for (index, command) in commands.iter().enumerate() {
                                        let bench_id: $crate::BenchmarkId = format!("{}_{}", bench.id, index).into();
                                        if let Err(message) = bench_id.validate() {
                                            errors.add(
                                                $crate::error::Error::BenchError(
                                                    module_path.to_owned(),
                                                    internal_group.id.clone(),
                                                    binary_benchmark.id.to_string(),
                                                    bench_id.to_string(),
                                                    message
                                                )
                                            );
                                            continue;
                                        }
                                        if !bench_ids.insert(bench_id.clone()) {
                                            errors.add(
                                                $crate::error::Error::BenchError(
                                                    module_path.to_owned(),
                                                    internal_group.id.clone(),
                                                    binary_benchmark.id.to_string(),
                                                    bench.id.to_string(),
                                                    format!("Duplicate id: '{}'", bench_id)
                                                )
                                            );
                                            continue;
                                        }
                                        let internal_bench =
                                            $crate::internal::InternalBinaryBenchmarkBench {
                                                id: Some(bench_id.into()),
                                                args: None,
                                                function_name: binary_benchmark.id.to_string(),
                                                command: command.into(),
                                                config: bench.config.clone(),
                                                has_setup: bench.setup.is_some()
                                                        || binary_benchmark.setup.is_some(),
                                                has_teardown: bench.teardown.is_some()
                                                        || binary_benchmark.teardown.is_some(),
                                        };
                                        internal_binary_benchmark.benches.push(internal_bench);
                                    }
                                }
                            }
                        }
                        internal_group.binary_benchmarks.push(internal_binary_benchmark);
                    }

                    internal_benchmark_groups.groups.push(internal_group);
                }
            )+

            if !errors.is_empty() {
                return Err(errors);
            }

            let encoded = $crate::bincode::serialize(&internal_benchmark_groups).expect("Encoded benchmark");
            let mut child = cmd
                .arg(encoded.len().to_string())
                .stdin(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run benchmarks. \
                    Is iai-callgrind-runner installed and iai-callgrind-runner in your $PATH?. \
                    You can also set the environment variable IAI_CALLGRIND_RUNNER to the \
                    absolute path of the iai-callgrind-runner executable.");

            let mut stdin = child.stdin.take().expect("Opening stdin to submit encoded benchmark");
            std::thread::spawn(move || {
                use std::io::Write;
                stdin.write_all(&encoded).expect("Writing encoded benchmark to stdin");
            });

            let status = child.wait().expect("Wait for child process to exit");
            if !status.success() {
                std::process::exit(1);
            }

            Ok(())
        }

        fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            $(
                __has_setup = true;
                if __run {
                    $setup;
                }
            )?
            __has_setup
        }

        fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            $(
                __has_teardown = true;
                if __run {
                    $teardown;
                }
            )?
            __has_teardown
        }

        fn main() {
            let mut args_iter = std::env::args().skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                let mut current = args_iter.next().expect("Expecting a function type");
                let next = args_iter.next();
                match (current.as_str(), next) {
                    ("setup", None) => {
                        __run_setup(true);
                    },
                    ("teardown", None) => {
                        __run_teardown(true);
                    },
                    $(
                        (group @ stringify!($group), Some(next)) => {
                            let current = next;
                            let next = args_iter.next();

                            match (current.as_str(), next) {
                                ("setup", None) => {
                                    $group::__run_setup(true);
                                },
                                ("teardown", None) => {
                                    $group::__run_teardown(true);
                                }
                                (key @ ("setup" | "teardown"), Some(next)) => {
                                    let group_index = next
                                            .parse::<usize>()
                                            .expect("The group index should be a number");
                                    let bench_index = args_iter
                                            .next()
                                            .expect("The bench index should be present")
                                            .parse::<usize>()
                                            .expect("The bench index should be a number");
                                    if key == "setup" {
                                        $group::__run_bench_setup(group_index, bench_index);
                                    } else {
                                        $group::__run_bench_teardown(group_index, bench_index);
                                    }
                                }
                                (name, _) => panic!("Invalid function '{}' in group '{}'", name, group)
                            }
                        }
                    )+
                    (name, _) => panic!("function '{}' not found in this scope", name)
                }
            } else {
                if let Err(errors) = __run() {
                    eprintln!("{errors}");
                    std::process::exit(1);
                }
            };
        }
    };
    (
        $( config = $config:expr; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        library_benchmark_groups =
    ) => {
        compile_error!("The library_benchmark_groups argument needs at least one `name` of a `library_benchmark_group!`");
    };
    (
        $( config = $config:expr ; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        library_benchmark_groups = $( $group:ident ),+ $(,)*
    ) => {
        #[inline(never)]
        fn __run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.13.4";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--lib-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(env!("CARGO_PKG_NAME"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable benchmark binary

            let mut config: Option<$crate::internal::InternalLibraryBenchmarkConfig> = None;
            $(
                config = Some($config.into());
            )?

            let mut internal_benchmark_groups = $crate::internal::InternalLibraryBenchmarkGroups {
                config: config.unwrap_or_default(),
                command_line_args: this_args.collect(),
                has_setup: __run_setup(false),
                has_teardown: __run_teardown(false),
                ..Default::default()
            };

            $(
                let mut internal_group = $crate::internal::InternalLibraryBenchmarkGroup {
                    id: stringify!($group).to_owned(),
                    config: $group::__get_config(),
                    compare_by_id: $group::__compare_by_id(),
                    library_benchmarks: vec![],
                    has_setup: $group::__run_setup(false),
                    has_teardown: $group::__run_teardown(false),
                };
                for (function_name, get_config, macro_lib_benches) in $group::__BENCHES {
                    let mut benches = $crate::internal::InternalLibraryBenchmarkBenches {
                        benches: vec![],
                        config: get_config()
                    };
                    for macro_lib_bench in macro_lib_benches.iter() {
                        let bench = $crate::internal::InternalLibraryBenchmarkBench {
                            id: macro_lib_bench.id_display.map(|i| i.to_string()),
                            args: macro_lib_bench.args_display.map(|i| i.to_string()),
                            function_name: function_name.to_string(),
                            config: macro_lib_bench.config.map(|f| f()),
                        };
                        benches.benches.push(bench);
                    }
                    internal_group.library_benchmarks.push(benches);
                }

                internal_benchmark_groups.groups.push(internal_group);
            )+

            let encoded = $crate::bincode::serialize(&internal_benchmark_groups).expect("Encoded benchmark");
            let mut child = cmd
                .arg(encoded.len().to_string())
                .stdin(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run benchmarks. \
                    Is iai-callgrind-runner installed and iai-callgrind-runner in your $PATH?. \
                    You can also set the environment variable IAI_CALLGRIND_RUNNER to the \
                    absolute path of the iai-callgrind-runner executable.");

            let mut stdin = child.stdin.take().expect("Opening stdin to submit encoded benchmark");
            std::thread::spawn(move || {
                use std::io::Write;
                stdin.write_all(&encoded).expect("Writing encoded benchmark to stdin");
            });

            let status = child.wait().expect("Wait for child process to exit");
            if !status.success() {
                std::process::exit(1);
            }
        }

        #[inline(never)]
        fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            $(
                __has_setup = true;
                if __run {
                    $setup;
                }
            )?
            __has_setup
        }

        #[inline(never)]
        fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            $(
                __has_teardown = true;
                if __run {
                    $teardown;
                }
            )?
            __has_teardown
        }

        fn main() {
            let mut args_iter = std::hint::black_box(std::env::args()).skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                let current = std::hint::black_box(args_iter.next().expect("Expecting a function type"));
                let next = std::hint::black_box(args_iter.next());
                match current.as_str() {
                    "setup" if next.is_none() => {
                        __run_setup(true);
                    },
                    "teardown" if next.is_none() => {
                        __run_teardown(true);
                    },
                    $(
                        stringify!($group) => {
                            match std::hint::black_box(
                                next
                                    .expect("An argument `setup`, `teardown` or an index should be present")
                                    .as_str()
                            ) {
                                "setup" => {
                                    $group::__run_setup(true);
                                },
                                "teardown" => {
                                    $group::__run_teardown(true);
                                }
                                value => {
                                    let group_index = std::hint::black_box(
                                        value
                                            .parse::<usize>()
                                            .expect("Expecting a valid group index")
                                    );
                                    let bench_index = std::hint::black_box(
                                        args_iter
                                            .next()
                                            .expect("A bench index should be present")
                                            .parse::<usize>()
                                            .expect("Expecting a valid bench index")
                                    );
                                    $group::__run(group_index, bench_index);
                                }
                            }
                        }
                    )+
                    name => panic!("function '{}' not found in this scope", name)
                }
            } else {
                std::hint::black_box(__run());
            };
        }
    };
    (
        callgrind_args = $( $args:literal ),* $(,)*; $(;)*
        functions = $( $func_name:ident ),+ $(,)*
    ) => {
        compile_error!(
            "You are using a deprecated syntax of the main! macro to set up library benchmarks. \
            See the README (https://github.com/iai-callgrind/iai-callgrind) and \
            docs (https://docs.rs/iai-callgrind/latest/iai_callgrind/) for further details."
        );
        pub fn main() {}
    };
    ( $( $func_name:ident ),+ $(,)* ) => {
        compile_error!(
            "You are using a deprecated syntax of the main! macro to set up library benchmarks. \
            See the README (https://github.com/iai-callgrind/iai-callgrind) and \
            docs (https://docs.rs/iai-callgrind/latest/iai_callgrind/) for further details."
        );
        pub fn main() {}
    };
}

/// Macro used to define a group of binary benchmarks
///
/// There are two apis to set up binary benchmarks. The recommended way is to [use the
/// `#[binary_benchmark]` attribute](#using-the-high-level-api-with-the-binary-benchmark-attribute).
/// But, if you find yourself in the situation that the attribute isn't enough you can fall back to
/// the [low level api](#the-low-level-api) or even [intermix both
/// styles](#intermixing-both-apis).
///
/// # The macro's arguments in detail:
///
/// The following top-level arguments are accepted (in this order):
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark, binary_benchmark_group, BinaryBenchmarkGroup, BinaryBenchmarkConfig};
/// # fn run_setup() {}
/// # fn run_teardown() {}
/// # #[binary_benchmark]
/// # fn bench_binary() -> iai_callgrind::Command { iai_callgrind::Command::new("some") }
/// binary_benchmark_group!(
///     name = my_group;
///     config = BinaryBenchmarkConfig::default();
///     compare_by_id = false;
///     setup = run_setup();
///     teardown = run_teardown();
///     benchmarks = bench_binary
/// );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
///
/// * __`name`__ (mandatory): A unique name used to identify the group for the `main!` macro
/// * __`config`__ (optional): A [`crate::BinaryBenchmarkConfig`]
/// * __`compare_by_id`__ (optional): The default is false. If true, all commands from the functions
///   specified in the `benchmarks` argument, are compared with each other as long as the ids (the
///   part after the `::` in `#[bench::id(...)]`) match.
/// * __`setup`__ (optional): A function which is executed before all benchmarks in this group
/// * __`teardown`__ (optional): A function which is executed after all benchmarks in this group
/// * __`benchmarks`__ (mandatory): A `,`-separated list of `#[binary_benchmark]` annotated function
///   names you want to put into this group. Or, if you want to use the low level api
///
///   `|IDENTIFIER: &mut BinaryBenchmarkGroup| EXPRESSION`
///
///   or the shorter `|IDENTIFIER| EXPRESSION`
///
///   where `IDENTIFIER` is the identifier of your choice for the `BinaryBenchmarkGroup` (we use
///   `group` throughout our examples) and `EXPRESSION` is the code where you make use of the
///   `BinaryBenchmarkGroup` to set up the binary benchmarks
///
/// # Using the high-level api with the `#[binary benchmark]` attribute
///
/// A small introductory example which demonstrates the basic setup (assuming a crate's binary is
/// named `my-foo`):
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup, binary_benchmark};
///
/// #[binary_benchmark]
/// #[bench::hello_world("hello world")]
/// #[bench::foo("foo")]
/// #[benches::multiple("bar", "baz")]
/// fn bench_binary(arg: &str) -> iai_callgrind::Command {
///      iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///          .arg(arg)
///          .build()
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = bench_binary
/// );
///
/// # fn main() {
/// iai_callgrind::main!(binary_benchmark_groups = my_group);
/// # }
/// ```
///
/// To be benchmarked a `binary_benchmark_group` has to be added to the `main!` macro by adding its
/// name to the `binary_benchmark_groups` argument of the `main!` macro. See there for further
/// details about the [`crate::main`] macro. See the documentation of [`crate::binary_benchmark`]
/// for more details about the attribute itself and the inner attributes `#[bench]` and
/// `#[benches]`.
///
/// # The low-level api
///
/// Using the low-level api has advantages but when it comes to stability in terms of usability, the
/// low level api might be considered less stable. What does this mean? If we have to make changes
/// to the inner workings of iai-callgrind which not necessarily change the high-level api it is
/// more likely that the low-level api has to be adjusted. This implies you might have to adjust
/// your benchmarks more often with a version update of `iai-callgrind`. Hence, it is recommended to
/// use the high-level api as much as possible and only use the low-level api under special
/// circumstances. You can also [intermix both styles](#intermixing-both-apis)!
///
/// The low-level api mirrors the high-level constructs as close as possible. The
/// [`crate::BinaryBenchmarkGroup`] is a special case, since we use the information from the
/// `binary_benchmark_group!` macro [arguments](#the-macros-arguments-in-detail) (__`name`__,
/// __`config`__, ...) to create the `BinaryBenchmarkGroup` and pass it to the `benchmarks`
/// argument.
///
/// That being said, here's the basic usage:
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{binary_benchmark_group, BinaryBenchmark, Bench};
///
/// binary_benchmark_group!(
///     // All the other options from the `binary_benchmark_group` are used as usual
///     name = my_group;
///
///     // Note there's also the shorter form `benchmarks = |group|` but in the examples we want
///     // to be more explicit
///     benchmarks = |group: &mut BinaryBenchmarkGroup| {
///
///         // We have chosen `group` to be our identifier but it can be anything
///         group.binary_benchmark(
///
///             // This is the equivalent of the `#[binary_benchmark]` attribute. The `id`
///             // mirrors the function name of the `#[binary_benchmark]` annotated function.
///             BinaryBenchmark::new("some_id")
///                 .bench(
///
///                     // The equivalent of the `#[bench]` attribute.
///                     Bench::new("my_bench_id")
///                         .command(
///
///                             // The `Command` stays the same
///                             iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///                                 .arg("foo").build()
///                         )
///                 )
///         )
///     }
/// );
/// # fn main() {}
/// ```
///
/// Depending on your IDE, it's nicer to work with the code after the `|group: &mut
/// BinaryBenchmarkGroup|` if it resides in a separate function rather than the macro itself as in
///
/// ```rust
/// use iai_callgrind::{binary_benchmark_group, BinaryBenchmark, Bench, BinaryBenchmarkGroup};
///
/// fn setup_my_group(group: &mut BinaryBenchmarkGroup) {
///     // Enjoy all the features of your IDE ...
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = |group: &mut BinaryBenchmarkGroup| setup_my_group(group)
/// );
/// # fn main() {}
/// ```
///
/// The list of all structs and macros used exclusively in the low-level api:
/// * [`crate::BinaryBenchmarkGroup`]
/// * [`crate::BinaryBenchmark`]: Mirrors the `#[binary_benchmark]` attribute
/// * [`crate::Bench`]: Mirrors the `#[bench]` attribute
/// * [`crate::binary_benchmark_attribute`]: Used to add a `#[binary_benchmark]` attributed function
///   in [`crate::BinaryBenchmarkGroup::binary_benchmark`]
/// * [`crate::BenchmarkId`]: The benchmark id is for example used in
///   [`crate::BinaryBenchmark::new`] and [`crate::Bench::new`]
///
/// Note there's no equivalent for the `#[benches]` attribute. The [`crate::Bench`] behaves exactly
/// as the `#[benches]` attribute if more than a single [`crate::Command`] is added.
///
/// # Intermixing both apis
///
/// For example, if you started with the `#[binary_benchmark]` attribute and noticed you are limited
/// by it to set up all the [`crate::Command`]s the way you want, you can intermix both styles:
///
/// ```rust
/// # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
/// use iai_callgrind::{
///     binary_benchmark, binary_benchmark_group, BinaryBenchmark, Bench, BinaryBenchmarkGroup,
///     binary_benchmark_attribute
/// };
///
/// #[binary_benchmark]
/// #[bench::foo("foo")]
/// #[benches::multiple("bar", "baz")]
/// fn bench_binary(arg: &str) -> iai_callgrind::Command {
///     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///         .arg(arg)
///         .build()
/// }
///
/// fn setup_my_group(group: &mut BinaryBenchmarkGroup) {
///     group
///         // Simply add what you already have with the `binary_benchmark_attribute!` macro.
///         // This macro returns a `BinaryBenchmark`, so you could even add more `Bench`es
///         // to it instead of creating a new one as we do below
///         .binary_benchmark(binary_benchmark_attribute!(bench_binary))
///         .binary_benchmark(
///             BinaryBenchmark::new("did_not_work_with_attribute")
///                 .bench(Bench::new("low_level")
///                     .command(
///                         iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
///                             .arg("foo")
///                             .build()
///                     )
///                 )
///         );
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmarks = |group: &mut BinaryBenchmarkGroup| setup_my_group(group)
/// );
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! binary_benchmark_group {
    (
        name = $name:ident; $(;)*
        $(before = $before:ident $(,bench = $bench_before:literal)? ; $(;)*)?
        $(after = $after:ident $(,bench = $bench_after:literal)? ; $(;)*)?
        $(setup = $setup:ident $(,bench = $bench_setup:literal)? ; $(;)*)?
        $(teardown = $teardown:ident $(,bench = $bench_teardown:literal)? ; $(;)*)?
        $( config = $config:expr ; $(;)* )?
        benchmark = |$cmd:literal, $group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        compile_error!(
            "You are using a deprecated syntax of the binary_benchmark_group! macro to set up binary \
            benchmarks. See the README (https://github.com/iai-callgrind/iai-callgrind), the \
            CHANGELOG on the same page and docs (https://docs.rs/iai-callgrind/latest/iai_callgrind) \
            for further details."
        );
    };
    (
        name = $name:ident; $(;)*
        $( before = $before:ident $(,bench = $bench_before:literal)? ; $(;)* )?
        $( after = $after:ident $(,bench = $bench_after:literal)? ; $(;)* )?
        $( setup = $setup:ident $(,bench = $bench_setup:literal)? ; $(;)* )?
        $( teardown = $teardown:ident $(,bench = $bench_teardown:literal )? ; $(;)* )?
        $( config = $config:expr ; $(;)* )?
        benchmark = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        compile_error!(
            "You are using a deprecated syntax of the binary_benchmark_group! macro to set up binary \
            benchmarks. See the README (https://github.com/iai-callgrind/iai-callgrind), the \
            CHANGELOG on the same page and docs (https://docs.rs/iai-callgrind/latest/iai_callgrind) \
            for further details."
        );
    };
    (
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = $( $function:ident ),+ $(,)*
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs a unique name. See the documentation of this macro for \
            further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = some_binary_benchmark);"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks =
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs at least 1 benchmark function which is annotated with \
            #[binary_benchmark] or you can use the low level syntax. See the documentation of this \
            macro for further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = some_binary_benchmark);"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs at least 1 benchmark function which is annotated with \
            #[binary_benchmark] or you can use the low level syntax. See the documentation of this \
            macro for further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = some_binary_benchmark);"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = $( $function:ident ),+ $(,)*
    ) => {
        pub mod $name {
            use super::*;

            pub const __IS_ATTRIBUTE: bool = true;

            pub const __BENCHES: &[&(
                &'static str,
                fn() -> Option<$crate::internal::InternalBinaryBenchmarkConfig>,
                &[$crate::internal::InternalMacroBinBench]
            )]= &[
                $(
                    &(
                        stringify!($function),
                        super::$function::__get_config,
                        super::$function::__BENCHES
                    )
                ),+
            ];

            pub fn __run_setup(__run: bool) -> bool {
                let mut __has_setup = false;
                $(
                    __has_setup = true;
                    if __run {
                        $setup;
                    }
                )?
                __has_setup
            }

            pub fn __run_teardown(__run: bool) -> bool {
                let mut __has_teardown = false;
                $(
                    __has_teardown = true;
                    if __run {
                        $teardown;
                    }
                )?
                __has_teardown
            }

            pub fn __compare_by_id() -> Option<bool> {
                let mut comp = None;
                $(
                    comp = Some($compare);
                )?
                comp
            }

            pub fn __get_config() -> Option<$crate::internal::InternalBinaryBenchmarkConfig> {
                let mut config = None;
                $(
                    config = Some($config.into());
                )?
                config
            }

            pub fn __run_bench_setup(group_index: usize, bench_index: usize) {
                if let Some(setup) = __BENCHES[group_index].2[bench_index].setup {
                    setup();
                };
            }

            pub fn __run_bench_teardown(group_index: usize, bench_index: usize) {
                if let Some(teardown) = __BENCHES[group_index].2[bench_index].teardown {
                    teardown();
                };
            }

            pub fn $name(_: &mut $crate::BinaryBenchmarkGroup) {}
        }
    };
    (
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs a unique name. See the documentation of this macro for \
            further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = |group: &mut BinaryBenchmarkGroup| ... );"
        );
    };
    (
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident| $body:expr
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs a unique name. See the documentation of this macro for \
            further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = |group| ... );"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident|
    ) => {
        compile_error!(
            "This low level form of the binary_benchmark_group! needs you to use the \
            `BinaryBenchmarkGroup` to setup benchmarks. See the documentation of this macro for \
            further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = |group| { \
                group.binary_benchmark(/* BinaryBenchmark::new */); });"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident: &mut BinaryBenchmarkGroup|
    ) => {
        compile_error!(
            "This low level form of the binary_benchmark_group! needs you to use the \
            `BinaryBenchmarkGroup` to setup benchmarks. See the documentation of this macro for \
            further details.\n\n\
            hint = binary_benchmark_group!(name = some_ident; benchmarks = |group: &mut \
                BinaryBenchmarkGroup| { group.binary_benchmark(/* BinaryBenchmark::new */); });"
        );
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        pub mod $name {
            use super::*;

            pub const __IS_ATTRIBUTE: bool = false;

            pub const __BENCHES: &[&(
                &'static str,
                fn() -> Option<$crate::internal::InternalBinaryBenchmarkConfig>,
                &[$crate::internal::InternalMacroBinBench]
            )]= &[];

            pub fn __run_setup(__run: bool) -> bool {
                let mut __has_setup = false;
                $(
                    __has_setup = true;
                    if __run {
                        $setup;
                    }
                )?
                __has_setup
            }

            pub fn __run_teardown(__run: bool) -> bool {
                let mut __has_teardown = false;
                $(
                    __has_teardown = true;
                    if __run {
                        $teardown;
                    }
                )?
                __has_teardown
            }

            pub fn __get_config() -> Option<$crate::internal::InternalBinaryBenchmarkConfig> {
                let mut config = None;
                $(
                    config = Some($config.into());
                )?
                config
            }

            pub fn __compare_by_id() -> Option<bool> {
                let mut comp = None;
                $(
                    comp = Some($compare);
                )?
                comp
            }

            pub fn __run_bench_setup(group_index: usize, bench_index: usize) {
                let mut group = $crate::BinaryBenchmarkGroup::default();
                $name(&mut group);

                let bench = group
                    .binary_benchmarks
                    .iter()
                    .nth(group_index)
                    .expect("The group index for setup should be present");
                // In the runner each command is a `BinBench` and it is the index of the command
                // which we're getting back from the runner. So, we have to iterate over the
                // commands of each Bench to extract the correct setup function.
                //
                // commands                           => bench_index => The correct setup function
                // bench.benches[0].commands = [a, b] => 0, 1        => bench.benches[0].setup
                // bench.benches[1].commands = [c]    => 2           => bench.benches[1].setup
                // bench.benches[2].commands = [d, e] => 3, 4        => bench.benches[2].setup
                //
                // We also need to take care of that there can be a global setup function
                // `BinaryBenchmark::setup`, which can be overridden by a `Bench::setup`
                if let Some(setup) = bench
                        .benches
                        .iter()
                        .flat_map(|b| b.commands.iter().map(|c| (b.setup, c)))
                        .nth(bench_index)
                        .map(|(setup, _)| setup)
                        .expect("The bench index for setup should be present") {
                    setup();
                } else if let Some(setup) = bench.setup {
                    setup();
                } else {
                    // This branch should be unreachable so we do nothing
                }
            }

            pub fn __run_bench_teardown(group_index: usize, bench_index: usize) {
                let mut group = $crate::BinaryBenchmarkGroup::default();
                $name(&mut group);

                let bench = group
                    .binary_benchmarks
                    .iter()
                    .nth(group_index)
                    .expect("The group index for teardown should be present");
                if let Some(teardown) = bench
                        .benches
                        .iter()
                        .flat_map(|b| b.commands.iter().map(|c| (b.teardown, c)))
                        .nth(bench_index)
                        .map(|(teardown, _)| teardown)
                        .expect("The bench index for teardown should be present") {
                    teardown();
                } else if let Some(teardown) = bench.teardown {
                    teardown();
                } else {
                    // This branch should be unreachable so we do nothing
                }
            }

            #[inline(never)]
            pub fn $name($group: &mut $crate::BinaryBenchmarkGroup) {
                $body;
            }
        }
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        benchmarks = |$group:ident| $body:expr
    ) => {
        binary_benchmark_group!(
            name = $name;
            $( config = $config; )?
            $( compare_by_id = $compare; )?
            $( setup = $setup; )?
            $( teardown = $teardown; )?
            benchmarks = |$group: &mut BinaryBenchmarkGroup| $body
        );
    };
}

/// Macro used to define a group of library benchmarks
///
/// A small introductory example which shows the basic setup. This macro only accepts benchmarks
/// annotated with `#[library_benchmark]` ([`crate::library_benchmark`]).
///
/// ```rust
/// use iai_callgrind::{library_benchmark_group, library_benchmark};
///
/// #[library_benchmark]
/// fn bench_something() -> u64 {
///     42
/// }
///
/// library_benchmark_group!(
///     name = my_group;
///     benchmarks = bench_something
/// );
///
/// # fn main() {
/// iai_callgrind::main!(library_benchmark_groups = my_group);
/// # }
/// ```
///
/// To be benchmarked a `library_benchmark_group` has to be added to the `main!` macro by adding its
/// name to the `library_benchmark_groups` argument of the `main!` macro. See there for further
/// details about the [`crate::main`] macro.
///
/// The following top-level arguments are accepted in this order:
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group, LibraryBenchmarkConfig};
/// # #[library_benchmark]
/// # fn some_func() {}
/// fn group_setup() {}
/// fn group_teardown() {}
/// library_benchmark_group!(
///     name = my_group;
///     config = LibraryBenchmarkConfig::default();
///     compare_by_id = false;
///     setup = group_setup();
///     teardown = group_teardown();
///     benchmarks = some_func
/// );
/// # fn main() {
/// # }
/// ```
///
/// * __`name`__ (mandatory): A unique name used to identify the group for the `main!` macro
/// * __`config`__ (optional): A [`crate::LibraryBenchmarkConfig`] which is applied to all
///   benchmarks within the same group.
/// * __`compare_by_id`__ (optional): The default is false. If true, all benches in the benchmark
///   functions specified with the `benchmarks` argument, across any benchmark groups, are compared
///   with each other as long as the ids (the part after the `::` in `#[bench::id(...)]`) match.
/// * __`setup`__ (optional): A setup function or any valid expression which is run before all
///   benchmarks of this group
/// * __`teardown`__ (optional): A teardown function or any valid expression which is run after all
///   benchmarks of this group
/// * __`benchmarks`__ (mandatory): A list of comma separated benchmark functions which must be
///   annotated with `#[library_benchmark]`
#[macro_export]
macro_rules! library_benchmark_group {
    (
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        benchmarks = $( $function:ident ),+
    ) => {
        compile_error!("A library_benchmark_group! needs a name\n\nlibrary_benchmark_group!(name = some_ident; benchmarks = ...);");
    };
    (
        name = $name:ident;
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        benchmarks =
    ) => {
        compile_error!(
            "A library_benchmark_group! needs at least 1 benchmark function \
            annotated with #[library_benchmark]\n\n\
            library_benchmark_group!(name = some_ident; benchmarks = some_library_benchmark);");
    };
    (
        name = $name:ident; $(;)*
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        $( setup = $setup:expr ; $(;)* )?
        $( teardown = $teardown:expr ; $(;)* )?
        benchmarks = $( $function:ident ),+ $(,)*
    ) => {
        mod $name {
            use super::*;

            pub const __BENCHES: &[&(
                &'static str,
                fn() -> Option<$crate::internal::InternalLibraryBenchmarkConfig>,
                &[$crate::internal::InternalMacroLibBench]
            )]= &[
                $(
                    &(
                        stringify!($function),
                        super::$function::__get_config,
                        super::$function::__BENCHES
                    )
                ),+
            ];

            #[inline(never)]
            pub fn __get_config() -> Option<$crate::internal::InternalLibraryBenchmarkConfig> {
                let mut config: Option<$crate::internal::InternalLibraryBenchmarkConfig> = None;
                $(
                    config = Some($config.into());
                )?
                config
            }

            #[inline(never)]
            pub fn __compare_by_id() -> Option<bool> {
                let mut comp = None;
                $(
                    comp = Some($compare);
                )?
                comp
            }

            #[inline(never)]
            pub fn __run_setup(__run: bool) -> bool {
                let mut __has_setup = false;
                $(
                    __has_setup = true;
                    if __run {
                        $setup;
                    }
                )?
                __has_setup
            }

            #[inline(never)]
            pub fn __run_teardown(__run: bool) -> bool {
                let mut __has_teardown = false;
                $(
                    __has_teardown = true;
                    if __run {
                        $teardown;
                    }
                )?
                __has_teardown
            }

            #[inline(never)]
            pub fn __run(group_index: usize, bench_index: usize) {
                (__BENCHES[group_index].2[bench_index].func)();
            }
        }
    };
}
