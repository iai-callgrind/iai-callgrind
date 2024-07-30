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
/// which accepts the following top-level arguments:
///
/// * __`library_benchmark_groups`__ (mandatory): The `name` of one or more
///   [`library_benchmark_group!`](crate::library_benchmark_group) macros.
/// * __`config`__ (optional): Optionally specify a [`crate::LibraryBenchmarkConfig`] valid for all
///   benchmark groups
///
/// A library benchmark consists of
/// [`library_benchmark_groups`](crate::library_benchmark_group) and  with
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
        binary_benchmark_groups =
    ) => {
        compile_error!("The binary_benchmark_groups argument needs at least one `name` of a `binary_benchmark_group!`");
    };
    (
        $( config = $config:expr; $(;)* )?
        binary_benchmark_groups = $( $group:ident ),+ $(,)*
    ) => {
        #[inline(never)]
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.12.0";

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
                ..Default::default()
            };

            $(
                let mut group = $crate::BinaryBenchmarkGroup::from(
                    $crate::internal::InternalBinaryBenchmarkGroup {
                        id: Some(stringify!($group).to_owned()),
                        cmd: None,
                        config: $group::get_config(),
                        benches: Vec::default(),
                        assists: Vec::default(),
                    }
                );
                let (prog, assists) = $group::$group(&mut group);

                let mut group: $crate::internal::InternalBinaryBenchmarkGroup = group.into();
                group.cmd = prog;
                group.assists = assists;

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

        fn main() {
            let mut args_iter = std::hint::black_box(std::env::args()).skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                match std::hint::black_box(args_iter.next().expect("Expecting a function type")).as_str() {
                    $(
                        concat!(stringify!($group), "::", "before") => $group::before(),
                        concat!(stringify!($group), "::", "after") => $group::after(),
                        concat!(stringify!($group), "::", "setup") => $group::setup(),
                        concat!(stringify!($group), "::", "teardown") => $group::teardown(),
                    )+
                    name => panic!("function '{}' not found in this scope", name)
                }
            } else {
                std::hint::black_box(run());
            };
        }
    };
    (
        $( config = $config:expr; $(;)* )?
        library_benchmark_groups =
    ) => {
        compile_error!("The library_benchmark_groups argument needs at least one `name` of a `library_benchmark_group!`");
    };
    (
        $( config = $config:expr ; $(;)* )?
        library_benchmark_groups = $( $group:ident ),+ $(,)*
    ) => {
        #[inline(never)]
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.12.0";

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
                ..Default::default()
            };

            $(
                let mut group = $crate::internal::InternalLibraryBenchmarkGroup {
                    id: Some(stringify!($group).to_owned()),
                    config: $group::get_config(),
                    compare: $group::compare(),
                    benches: vec![]
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

        fn main() {
            let mut args_iter = std::hint::black_box(std::env::args()).skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                match std::hint::black_box(args_iter.next().expect("Expecting a function type")).as_str() {
                    $(
                        stringify!($group) => {
                            let group_index = std::hint::black_box(
                                args_iter
                                    .next()
                                    .expect("Expecting a group index")
                                    .parse::<usize>()
                                    .expect("Expecting a valid group index")
                            );
                            let bench_index = std::hint::black_box(
                                args_iter
                                    .next()
                                    .expect("Expecting a bench index")
                                    .parse::<usize>()
                                    .expect("Expecting a valid bench index")
                            );
                            $group::run(group_index, bench_index);
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
        name = $name:ident; $(;)*
        $(before = $before:ident $(,bench = $bench_before:literal)? ; $(;)*)?
        $(after = $after:ident $(,bench = $bench_after:literal)? ; $(;)*)?
        $(setup = $setup:ident $(,bench = $bench_setup:literal)? ; $(;)*)?
        $(teardown = $teardown:ident $(,bench = $bench_teardown:literal)? ; $(;)*)?
        $( config = $config:expr ; $(;)* )?
        benchmark = |$cmd:expr, $group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        pub mod $name {
            #[inline(never)]
            pub fn before() {
                $(
                    let _ = std::hint::black_box(super::$before());
                )?
            }

            #[inline(never)]
            pub fn after() {
                $(
                    let _ = std::hint::black_box(super::$after());
                )?
            }

            #[inline(never)]
            pub fn setup() {
                $(
                    let _ = std::hint::black_box(super::$setup());
                )?
            }

            #[inline(never)]
            pub fn teardown() {
                $(
                    let _ = std::hint::black_box(super::$teardown());
                )?
            }

            #[inline(never)]
            pub fn get_config() -> Option<$crate::internal::InternalBinaryBenchmarkConfig> {
                use super::*;

                let mut config = None;
                $(
                    config = Some($config.into());
                )?
                config
            }

            #[inline(never)]
            pub fn $name($group: &mut $crate::BinaryBenchmarkGroup) ->
                (Option<$crate::internal::InternalCmd>, Vec<$crate::internal::InternalAssistant>)
            {
                let cmd: &str = $cmd;
                let cmd = (!cmd.is_empty()).then(|| $crate::internal::InternalCmd {
                        display: cmd.to_owned(),
                        cmd: option_env!(concat!("CARGO_BIN_EXE_", $cmd)).unwrap_or(cmd).to_owned()
                    }
                );

                let mut assists: Vec<$crate::internal::InternalAssistant> = vec![];
                $(
                    let mut bench_before = false;
                    $(
                        bench_before = $bench_before;
                    )?
                    assists.push($crate::internal::InternalAssistant {
                        id: "before".to_owned(),
                        name: stringify!($before).to_owned(),
                        bench: bench_before
                    });
                )?
                $(
                    let mut bench_after = false;
                    $(
                        bench_after = $bench_after;
                    )?
                    assists.push($crate::internal::InternalAssistant {
                        id: "after".to_owned(),
                        name: stringify!($after).to_owned(),
                        bench: bench_after
                    });
                )?
                $(
                    let mut bench_setup = false;
                    $(
                        bench_setup = $bench_setup;
                    )?
                    assists.push($crate::internal::InternalAssistant {
                        id: "setup".to_owned(),
                        name: stringify!($setup).to_owned(),
                        bench: bench_setup
                    });
                )?
                $(
                    let mut bench_teardown = false;
                    $(
                        bench_teardown = $bench_teardown;
                    )?
                    assists.push($crate::internal::InternalAssistant {
                        id: "teardown".to_owned(),
                        name: stringify!($teardown).to_owned(),
                        bench: bench_teardown
                    });
                )?

                use super::*;
                $body;

                (cmd, assists)
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
/// The following top-level arguments are accepted:
///
/// ```rust
/// # use iai_callgrind::{library_benchmark, library_benchmark_group, LibraryBenchmarkConfig};
/// # #[library_benchmark]
/// # fn some_func() {}
/// library_benchmark_group!(
///     name = my_group;
///     config = LibraryBenchmarkConfig::default();
///     compare_by_id = false;
///     benchmarks = some_func
/// );
/// # fn main() {
/// # }
/// ```
///
/// * `__name__` (mandatory): A unique name used to identify the group for the `main!` macro
/// * `__config__` (optional): A [`crate::LibraryBenchmarkConfig`] which is applied to all
///   benchmarks within the same group.
/// * `__compare_by_id__` (optional): The default is false. If true, all benches in the benchmark
///   functions specified with the `benchmarks` argument, across any benchmark groups, are compared
///   with each other as long as the ids (the part after the `::` in `#[bench::id(...)]`) match.
#[macro_export]
macro_rules! library_benchmark_group {
    (
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
        benchmarks = $( $function:ident ),+
    ) => {
        compile_error!("A library_benchmark_group! needs a name\n\nlibrary_benchmark_group!(name = some_ident; benchmarks = ...);");
    };
    (
        name = $name:ident;
        $( config = $config:expr ; $(;)* )?
        $( compare_by_id = $compare:literal ; $(;)* )?
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
            pub fn run(group_index: usize, bench_index: usize) {
                (BENCHES[group_index].2[bench_index].func)();
            }
        }
    };
}
