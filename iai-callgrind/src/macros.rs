//! Contains macros which together define a benchmark harness that can be used in place of the
//! standard benchmark harness. This allows the user to run Iai benchmarks with `cargo bench`.

/// The `iai_callgrind::main` macro expands to a `main` function which runs all of the benchmarks.
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
/// The scheme to setup binary benchmarks makes use of [`crate::binary_benchmark_group`]
/// and [`crate::BinaryBenchmarkGroup`] to set up benches with [`crate::Run`] roughly
/// looking like this:
///
/// ```rust
/// use iai_callgrind::{main, binary_benchmark_group, Run, Arg};
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
///         group
///         .bench(Run::with_arg(Arg::new(
///             "positional arguments",
///             ["foo", "foo bar"],
///         )))
///         .bench(Run::with_arg(Arg::empty("no argument")));
///     }
/// );
///
/// # fn main() {
/// main!(binary_benchmark_groups = my_group);
/// # }
/// ```
///
/// See the documentation of [`crate::binary_benchmark_group`] and [`crate::Run`] for more
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

        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.12.1";

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


            let mut benchmark = $crate::internal::InternalBinaryBenchmark {
                config: config.unwrap_or_default(),
                command_line_args: this_args.collect(),
                has_setup: __run_setup(false),
                has_teardown: __run_teardown(false),
                ..Default::default()
            };

            $(
                if $group::__IS_ATTRIBUTE {
                    let mut group = $crate::internal::InternalBinaryBenchmarkGroup {
                        id: stringify!($group).to_owned(),
                        config: $group::__get_config(),
                        benches: vec![],
                        has_setup: $group::__run_setup(false),
                        has_teardown: $group::__run_teardown(false),
                    };
                    for (bench_name, get_config, macro_bin_benches) in $group::__BENCHES {
                        let mut benches = $crate::internal::InternalBinaryBenchmarkBenches {
                            benches: vec![],
                            config: get_config()
                        };
                        for macro_bin_bench in macro_bin_benches.iter() {
                            let bench = $crate::internal::InternalBinaryBenchmarkBench {
                                id: macro_bin_bench.id_display.map(|i| i.to_string()),
                                args: macro_bin_bench.args_display.map(|i| i.to_string()),
                                bench: bench_name.to_string(),
                                command: (macro_bin_bench.func)().into(),
                                config: macro_bin_bench.config.map(|f| f()),
                                has_setup: macro_bin_bench.setup.is_some(),
                                has_teardown: macro_bin_bench.teardown.is_some()
                            };
                            benches.benches.push(bench);
                        }
                        group.benches.push(benches);
                    }

                    benchmark.groups.push(group);
                } else {
                    let mut group = $crate::BinaryBenchmarkGroup::default();
                    $group::$group(&mut group);

                    let mut internal_group = $crate::internal::InternalBinaryBenchmarkGroup {
                        id: stringify!($group).to_owned(),
                        config: $group::__get_config(),
                        benches: vec![],
                        has_setup: $group::__run_setup(false),
                        has_teardown: $group::__run_teardown(false),
                    };

                    let mut binary_benchmark_ids =
                        std::collections::HashSet::<$crate::BenchmarkId>::new();
                    for binary_benchmark in group.binary_benchmarks {
                        if !binary_benchmark_ids.insert(binary_benchmark.id.clone()) {
                            panic!("Duplicate binary benchmark id: {}", &binary_benchmark.id);
                        }

                        let mut internal_binary_benchmarks =
                            $crate::internal::InternalBinaryBenchmarkBenches {
                                benches: vec![],
                                config: binary_benchmark.config.map(Into::into)
                        };

                        let mut bench_ids =
                            std::collections::HashSet::<$crate::BenchmarkId>::new();

                        for bench in binary_benchmark.benches {
                            match bench.commands.as_slice() {
                                [] => {
                                    panic!("Missing command for bench with id: {}", bench.id);
                                },
                                [command] => {
                                    if !bench_ids.insert(bench.id.clone()) {
                                        panic!("Duplicate bench id: {}", bench.id);
                                    }
                                    let internal_bench =
                                        $crate::internal::InternalBinaryBenchmarkBench {
                                            id: Some(bench.id.into()),
                                            args: None,
                                            bench: binary_benchmark.id.clone().into(),
                                            command: command.into(),
                                            config: bench.config.map(Into::into),
                                            has_setup: bench.setup.is_some()
                                                    || binary_benchmark.setup.is_some(),
                                            has_teardown: bench.teardown.is_some()
                                                    || binary_benchmark.teardown.is_some(),
                                    };
                                    internal_binary_benchmarks.benches.push(internal_bench);
                                },
                                commands => {
                                    for (index, command) in commands.iter().enumerate() {
                                        let bench_id: $crate::BenchmarkId = format!("{}_{}", bench.id, index).into();
                                        if !bench_ids.insert(bench_id.clone()) {
                                            panic!("Duplicate bench id: {}", bench.id);
                                        }
                                        let internal_bench =
                                            $crate::internal::InternalBinaryBenchmarkBench {
                                                id: Some(bench_id.into()),
                                                args: None,
                                                bench: String::new(),
                                                command: command.into(),
                                                config: bench.config.as_ref().map(Into::into),
                                                has_setup: bench.setup.is_some(),
                                                has_teardown: bench.teardown.is_some(),
                                        };
                                        internal_binary_benchmarks.benches.push(internal_bench);
                                    }
                                }
                            }
                        }
                        internal_group.benches.push(internal_binary_benchmarks);
                    }

                    benchmark.groups.push(internal_group);
                }
            )+

            let encoded = $crate::bincode::serialize(&benchmark).expect("Encoded benchmark");
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
                                (name, _)=> panic!("Invalid function '{}' in group '{}'", name, group)
                            }
                        }
                    )+
                    (name, _) => panic!("function '{}' not found in this scope", name)
                }
            } else {
                run();
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
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.12.1";

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

            let mut benchmark = $crate::internal::InternalLibraryBenchmark {
                config: config.unwrap_or_default(),
                command_line_args: this_args.collect(),
                has_setup: __run_setup(false),
                has_teardown: __run_teardown(false),
                ..Default::default()
            };

            $(
                let mut group = $crate::internal::InternalLibraryBenchmarkGroup {
                    id: Some(stringify!($group).to_owned()),
                    config: $group::get_config(),
                    compare: $group::compare(),
                    benches: vec![],
                    has_setup: $group::run_setup(false),
                    has_teardown: $group::run_teardown(false),
                };
                for (bench_name, get_config, macro_lib_benches) in $group::BENCHES {
                    let mut benches = $crate::internal::InternalLibraryBenchmarkBenches {
                        benches: vec![],
                        config: get_config()
                    };
                    for macro_lib_bench in macro_lib_benches.iter() {
                        let bench = $crate::internal::InternalLibraryBenchmarkBench {
                            id: macro_lib_bench.id_display.map(|i| i.to_string()),
                            args: macro_lib_bench.args_display.map(|i| i.to_string()),
                            bench: bench_name.to_string(),
                            config: macro_lib_bench.config.map(|f| f()),
                        };
                        benches.benches.push(bench);
                    }
                    group.benches.push(benches);
                }

                benchmark.groups.push(group);
            )+

            let encoded = $crate::bincode::serialize(&benchmark).expect("Encoded benchmark");
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
                                    $group::run_setup(true);
                                },
                                "teardown" => {
                                    $group::run_teardown(true);
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
                                    $group::run(group_index, bench_index);
                                }
                            }
                        }
                    )+
                    name => panic!("function '{}' not found in this scope", name)
                }
            } else {
                std::hint::black_box(run());
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
/// A small introductory example which shows the basic setup:
///
/// ```rust
/// use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup};
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmark = |group: &mut BinaryBenchmarkGroup| {
///         // code to setup and configure the benchmarks in a group
///     }
/// );
///
/// iai_callgrind::main!(binary_benchmark_groups = my_group);
/// ```
///
/// To be benchmarked a `binary_benchmark_group` has to be added to the `main!` macro by adding its
/// name to the `binary_benchmark_groups` argument of the `main!` macro. See there for further
/// details about the [`crate::main`] macro.
///
/// This macro accepts two forms which slightly differ in the `benchmark` argument. In general, each
/// group shares the same `before`, `after`, `setup` and `teardown` functions, [`crate::Fixtures`]
/// and [`crate::BinaryBenchmarkConfig`].
///
/// The following top-level arguments are accepted:
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup, BinaryBenchmarkConfig};
/// # fn run_before() {}
/// # fn run_after() {}
/// # fn run_setup() {}
/// # fn run_teardown() {}
/// binary_benchmark_group!(
///     name = my_group;
///     before = run_before;
///     after = run_after;
///     setup = run_setup;
///     teardown = run_teardown;
///     config = BinaryBenchmarkConfig::default();
///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
///         // code to setup and configure the benchmarks in a group
///     }
/// );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
///
/// * __name__ (mandatory): A unique name used to identify the group for the `main!` macro
/// * __before__ (optional): A function which is run before all benchmarks
/// * __after__ (optional): A function which is run after all benchmarks
/// * __setup__ (optional): A function which is run before any benchmarks
/// * __teardown__ (optional): A function which is run before any benchmarks
/// * __config__ (optional): A [`crate::BinaryBenchmarkConfig`]
///
/// The `before`, `after`, `setup` and `teardown` arguments accept an additional argument `bench =
/// bool`
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup};
/// # fn run_before() {}
/// # binary_benchmark_group!(
/// # name = my_group;
/// before = run_before, bench = true;
/// #    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
/// #    }
/// # );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
///
/// which enables benchmarking of the respective function if wished so. Note that setup and teardown
/// functions are benchmarked only once the first time they are invoked, much like the before and
/// after functions. However, both functions are run as usual before or after any benchmark.
///
/// Only the `benchmark` argument differs. In the first form
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup, Run, Arg};
/// # binary_benchmark_group!(
/// # name = my_group;
/// benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
///     group.bench(Run::with_arg(Arg::new("some id", &["--foo=bar"])))
/// }
/// # );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
///
/// it accepts a `command` which is the default for all [`crate::Run`] of the same benchmark group.
/// This `command` supports auto-discovery of a crate's binary. For example if a crate's binary is
/// named `my-exe` then it is sufficient to pass `"my-exe"` to the benchmark argument as shown
/// above.
///
/// In the second form:
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup, Run, Arg};
/// # binary_benchmark_group!(
/// # name = my_group;
/// benchmark = |group: &mut BinaryBenchmarkGroup| {
///     // Usually, you should use `env!("CARGO_BIN_EXE_my-exe")` instead of an absolute path to a
///     // crate's binary
///     group.bench(Run::with_cmd(
///         "/path/to/my-exe",
///         Arg::new("some id", &["--foo=bar"]),
///     ))
/// }
/// # );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
///
/// the command can be left out and each [`crate::Run`] of a benchmark group has to define a `cmd`
/// by itself. Note that [`crate::Run`] does not support auto-discovery of a crate's binary.
///
/// If you feel uncomfortable working within the macro you can simply move the code to setup the
/// group's benchmarks into a separate function like so
///
/// ```rust
/// use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup, Run, Arg};
///
/// fn setup_my_group(group: &mut BinaryBenchmarkGroup) {
///     group.bench(Run::with_arg(Arg::new("some id", &["--foo=bar"])));
/// }
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| setup_my_group(group)
/// );
/// # fn main() {
/// # my_group::my_group(&mut BinaryBenchmarkGroup::default());
/// # }
/// ```
#[macro_export]
macro_rules! binary_benchmark_group {
    // TODO: DEPRECATE OLD SYNTAX
    (
        $( config = $config:expr ; $(;)* )?
        benchmark = |$cmd:expr, $group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        compile_error!("A binary_benchmark_group! needs a name\n\nbinary_benchmark_group!(name = some_ident; benchmark = ...);");
    };
    (
        $( config = $config:expr ; $(;)* )?
        benchmark = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        compile_error!("A binary_benchmark_group! needs a name\n\nbinary_benchmark_group!(name = some_ident; benchmark = ...);");
    };
    (
        name = $name:ident;
        $( config = $config:expr ; $(;)* )?
        benchmark =
    ) => {
        compile_error!(
            r#"A binary_benchmark_group! needs an expression specifying `BinaryBenchmarkGroup`:
binary_benchmark_group!(name = some_ident; benchmark = |group: &mut BinaryBenchmarkGroup| ... );
OR
binary_benchmark_group!(name = some_ident; benchmark = |"my_exe", group: &mut BinaryBenchmarkGroup| ... );
"#);
    };
    (
        name = $name:ident;
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        benchmarks =
    ) => {
        compile_error!(
            "A binary_benchmark_group! needs at least 1 benchmark function \
            annotated with #[binary_benchmark]\n\n\
            binary_benchmark_group!(name = some_ident; benchmarks = some_library_benchmark);");
    };
    (
        name = $name:ident; $(;)*
        $( setup = $setup:expr $(,bench = $bench_setup:literal)? ; $(;)* )?
        $( teardown = $teardown:expr $(,bench = $bench_teardown:literal)? ; $(;)* )?
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        benchmarks = $( $function:ident ),+ $(,)*
    ) => {
        // TODO: IMPLEMENT
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
        name = $name:ident; $(;)*
        $( setup = $setup:expr; $(;)* )?
        $( teardown = $teardown:expr; $(;)* )?
        $( config = $config:expr; $(;)* )?
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

            pub fn __run_bench_setup(group_index: usize, bench_index: usize) {
                let mut group = $crate::BinaryBenchmarkGroup::default();
                $name(&mut group);

                let bench = group
                    .binary_benchmarks
                    .iter()
                    .enumerate()
                    .find_map(|(i, b)| (i == group_index).then_some(b))
                    .expect("The group index for setup should be present");
                if let Some(setup) = bench
                        .benches
                        .iter()
                        .enumerate()
                        .find_map(|(i, b)| (i == bench_index).then_some(b.setup))
                        .expect("The bench index for setup should be present") {
                    setup();
                } else if let Some(setup) = bench.setup {
                    setup();
                } else {
                    // do nothing
                }
            }

            pub fn __run_bench_teardown(group_index: usize, bench_index: usize) {
                let mut group = $crate::BinaryBenchmarkGroup::default();
                $name(&mut group);

                let bench = group
                    .binary_benchmarks
                    .iter()
                    .enumerate()
                    .find_map(|(i, b)| (i == group_index).then_some(b))
                    .expect("The group index for teardown should be present");
                if let Some(teardown) = bench
                        .benches
                        .iter()
                        .enumerate()
                        .find_map(|(i, b)| (i == bench_index).then_some(b.teardown))
                        .expect("The bench index for teardown should be present") {
                    teardown();
                } else if let Some(teardown) = bench.teardown {
                    teardown();
                } else {
                    // do nothing
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
        $(before = $before:ident $(,bench = $bench_before:literal)? ; $(;)*)?
        $(after = $after:ident $(,bench = $bench_after:literal)? ; $(;)*)?
        $(setup = $setup:ident $(,bench = $bench_setup:literal)? ; $(;)*)?
        $(teardown = $teardown:ident $(,bench = $bench_teardown:literal)? ; $(;)*)?
        $( config = $config:expr ; $(;)* )?
        benchmark = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        $crate::binary_benchmark_group!(
            name = $name;
            $(before = $before $(,bench = $bench_before)?;)?
            $(after = $after $(,bench = $bench_after)?;)?
            $(setup = $setup $(,bench = $bench_setup)?;)?
            $(teardown = $teardown $(,bench = $bench_teardown)?;)?
            $( config = $config; )?
            benchmark = |"", $group: &mut BinaryBenchmarkGroup| $body
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

            pub const BENCHES: &[&(
                &'static str,
                fn() -> Option<$crate::internal::InternalLibraryBenchmarkConfig>,
                &[$crate::internal::InternalMacroLibBench]
            )]= &[
                $(
                    &(
                        stringify!($function),
                        super::$function::get_config,
                        super::$function::BENCHES
                    )
                ),+
            ];

            #[inline(never)]
            pub fn get_config() -> Option<$crate::internal::InternalLibraryBenchmarkConfig> {
                let mut config: Option<$crate::internal::InternalLibraryBenchmarkConfig> = None;
                $(
                    config = Some($config.into());
                )?
                config
            }

            #[inline(never)]
            pub fn compare() -> bool {
                let mut comp: bool = false;
                $(
                    comp = $compare;
                )?
                comp
            }

            #[inline(never)]
            pub fn run_setup(__run: bool) -> bool {
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
            pub fn run_teardown(__run: bool) -> bool {
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
            pub fn run(group_index: usize, bench_index: usize) {
                (BENCHES[group_index].2[bench_index].func)();
            }
        }
    };
}
