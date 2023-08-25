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
/// installed with the binary somewhere in your $PATH for example with
///
/// ```shell
/// cargo install iai-callgrind-runner
/// ```
///
/// `my_bench` has to be a rust file inside the 'benches' directory.
///
/// # Library Benchmarks
///
/// The [`crate::main`] macro has two forms to run library benchmarks:
///
/// ```ignore
/// main!(func1, func2, ...);
/// ```
///
/// which let's you specify benchmarking functions (func1, func2, ...) which are functions within
/// the same file like so:
///
/// ```rust
/// use iai_callgrind::{black_box, main};
///
/// fn fibonacci(n: u64) -> u64 {
///     match n {
///         0 => 1,
///         1 => 1,
///         n => fibonacci(n - 1) + fibonacci(n - 2),
///     }
/// }
///
/// #[inline(never)] // required for benchmarking functions
/// fn iai_benchmark_short() -> u64 {
///     fibonacci(black_box(10))
/// }
///
/// #[inline(never)] // required for benchmarking functions
/// fn iai_benchmark_long() -> u64 {
///     fibonacci(black_box(30))
/// }
///
/// # fn main() {
/// main!(iai_benchmark_short, iai_benchmark_long);
/// # }
/// ```
///
/// The second form has and additional parameter `callgrind_args`:
///
/// ```ignore
/// main!(
///     callgrind_args = "--arg-with-flags=yes", "arg-without-flags=is_ok_too"
///     functions = func1, func2
/// )
/// ```
///
/// if you need to pass arguments to valgrind's callgrind. See also [Callgrind Command-line
/// options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options). For an in-depth
/// description of library benchmarks and more examples see the [README#Library
/// Benchmarks](https://github.com/Joining7943/iai-callgrind#library-benchmarks) of this crate.
///
/// # Binary Benchmarks
///
/// There are two different ways to setup binary benchmarks. The recommended way is to use
/// [`crate::binary_benchmark_group`] and the main macro form with the `benchmark_binary_groups`
/// argument roughly looking like this:
///
/// ```rust
/// use iai_callgrind::{main, binary_benchmark_group};
///
/// binary_benchmark_group!(
///     name = my_group;
///     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
///         // code to setup benchmark runs goes here
///     }
/// );
///
/// # fn main() {
/// main!(binary_benchmark_groups = my_group);
/// # }
/// ```
///
/// See the documentation of [`crate::binary_benchmark_group`] for more details.
///
/// The second way uses just the `main` macro. It may lack the convenience and also some features of
/// the builder like api from above via the [`crate::binary_benchmark_group`] macro.
///
/// The `main` macro api for binary benchmarks allows the following top-level arguments:
///
/// ```rust,ignore
/// main!(
///     options = "--callgrind-argument=yes";
///     before = function_running_before_all_benchmarks;
///     after = function_running_after_all_benchmarks;
///     setup = function_running_before_any_benchmark;
///     teardown = function_running_after_any_benchmark;
///     sandbox = true;
///     fixtures = "path/to/fixtures";
///     run = cmd = "benchmark-tests", args = [];
/// )
/// ```
///
/// Here, `benchmark-tests` is an example of the name of the binary of a crate and it is assumed
/// that the `function_running_before_all_benchmarks` ... functions are defined somewhere in the
/// same file of the `main` macro. All top-level arguments must be separated by a `;`. However, only
/// `run` is mandatory. All other top-level arguments (like `options`, `setup` etc.) are optional.
///
/// ### `run` (Mandatory)
///
/// The `run` argument can be specified multiple times separated by a `;` but must be given at least
/// once. It takes the following arguments:
///
/// #### `cmd` (Mandatory)
///
/// This argument is allowed only once and specifies the name of one of the executables of the
/// benchmarked crate. The path of the executable is discovered automatically, so the name of the
/// `[[bin]]` as specified in the crate's `Cargo.toml` file is sufficient. The auto discovery
/// supports running the benchmarks with different profiles.
///
/// Although not the main purpose of `iai-callgrind`, it's possible to benchmark any executable in
/// the `PATH` or specified with an absolute path.
///
/// #### `args` (Mandatory)
///
/// The `args` argument must be specified at least once containing the arguments for the benchmarked
/// `cmd`. It can be an empty array `[]` to run to the [`cmd`](#cmd-mandatory) without any
/// arguments. Each `args` must have a unique `id`.
///
/// Specifying `args` multiple times (separated by a `,`) like so:
///
/// ```rust,ignore
/// main!(
///     run = cmd = "benchmark-tests",
///         id = "long", args = ["something"],
///         id = "short", args = ["other"]
/// )
/// ```
///
/// is a short-hand for specifying [`run`](#run-mandatory) with the same [`cmd`](#cmd-mandatory),
/// [`opts`](#opts-optional) and [`envs`](#envs-optional) arguments multiple times:
///
/// ```rust,ignore
/// main!(
///     run = cmd = "benchmark-tests", id = "long", args = ["something"];
///     run = cmd = "benchmark-tests", id = "short", args = ["other"]
/// )
/// ```
///
/// The output of a bench run with ids could look like:
///
/// ```text
/// test_bin_bench long:benchmark-tests something
///   Instructions:              322637 (No Change)
///   L1 Data Hits:              106807 (No Change)
///   L2 Hits:                      708 (No Change)
///   RAM Hits:                    3799 (No Change)
///   Total read+write:          433951 (No Change)
///   Estimated Cycles:          565949 (No Change)
/// test_bin_bench short:benchmark-tests other
///   Instructions:              155637 (No Change)
///   L1 Data Hits:              106807 (No Change)
///   L2 Hits:                      708 (No Change)
///   RAM Hits:                    3799 (No Change)
///   Total read+write:          433951 (No Change)
///   Estimated Cycles:          565949 (No Change)
/// ```
///
/// ###### `opts` (Optional)
///
/// `opts` is optional and can be specified once for every `run` and [`cmd`](#cmd-mandatory):
///
/// ```rust,ignore
/// main!(
///     run = cmd = "benchmark-tests",
///         opts = Options::default().env_clear(false),
///         args = ["something"];
/// )
/// ```
///
/// See the docs of [`crate::Options`] for more details.
///
///
/// #### `envs` (Optional)
///
/// `envs` may be used to set environment variables available in the `cmd`. This argument is
/// optional and can be specified once for every [`cmd`](#cmd-mandatory). There must be at least one
/// `KEY=VALUE` pair or `KEY` present in the array:
///
/// ```rust,ignore
/// main!(
///     run = cmd = "benchmark-tests",
///         envs = ["MY_VAR=SOME_VALUE", "MY_OTHER_VAR=VALUE"],
///         args = ["something"];
/// )
/// ```
///
/// See also the docs of [`crate::Run::env`] for more details.
///
/// ##### `sandbox` (Optional)
///
/// Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown` functions
/// are executed in a temporary directory.
///
/// ```rust,ignore
/// main!(
///     sandbox = true;
///     run = cmd = "benchmark-tests",
///         opts = Options::default().env_clear(false),
///         args = ["something"];
/// )
/// ```
///
/// See also the docs of [`crate::BinaryBenchmarkGroup::sandbox`] for more details.
///
/// ##### `options` (Optional)
///
/// A `,` separated list of strings which contain options for all `callgrind` invocations and
/// therefore benchmarked `cmd`s (Including benchmarked `before`, `after`, `setup` and `teardown`
/// functions).
///
/// ```rust,ignore
/// main!(
///     options = "--zero-before=benchmark_tests::main";
///     run = cmd = "benchmark-tests", args = [];
/// )
/// ```
///
/// See also [Passing arguments to callgrind](#passing-arguments-to-callgrind) and the documentation
/// of [Callgrind](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)
///
/// ##### `before`, `after`, `setup`, `teardown` (Optional)
///
/// Each of the `before`, `after`, `setup` and `teardown` top-level arguments is optional. If given,
/// this argument must specify a function of the benchmark file. These functions are meant to setup
/// and cleanup the benchmarks. Each function is invoked at a different stage of the benchmarking
/// process.
///
/// - `before`: This function is run once before all benchmarked `cmd`s
/// - `after`: This function is run once after all benchmarked `cmd`s
/// - `setup`: This function is run once before any benchmarked `cmd`
/// - `teardown`: This function is run once after any benchmarked `cmd`
///
/// See also the docs of [`crate::binary_benchmark_group`] for more details.
///
/// ##### `fixtures` (Optional)
///
/// The `fixtures` argument specifies a path to a directory containing fixtures which you want to be
/// available for all benchmarks and the `before`, `after`, `setup` and `teardown` functions. Per
/// default, the fixtures directory will be copied as is into the workspace directory of the
/// benchmark and following symlinks is switched off. The fixtures argument takes an additional
/// argument `follow_symlinks = bool`. If set to `true` and your fixtures directory contains
/// symlinks, these symlinks are resolved and instead of the symlink the target file or directory
/// will be copied into the fixtures directory.
///
/// ```rust,ignore
/// main!(
///     fixtures = "benches/fixtures", follow_symlinks = true;
///     run = cmd = "benchmark-tests", args = [];
/// )
/// ```
///
/// For more details about the functionality see the docs of [`crate::Fixtures`].
#[macro_export]
macro_rules! main {
    // TODO: CHANGE options to config and use BinaryBenchmarkConfig
    ( $( options = $( $options:literal ),+ $(,)*; )?
      $( before = $before:ident $(, bench = $bench_before:literal )? ; )?
      $( after = $after:ident $(, bench = $bench_after:literal )? ; )?
      $( setup = $setup:ident $(, bench = $bench_setup:literal )? ; )?
      $( teardown = $teardown:ident $(, bench = $bench_teardown:literal )? ; )?
      $( sandbox = $sandbox:literal; )?
      $( fixtures = $fixtures:literal $(, follow_symlinks = $follow_symlinks:literal )? ; )?
      $( run = cmd = $cmd:expr
            $(, envs = [ $( $envs:literal ),* $(,)* ] )?
            $(, opts = $opt:expr )? ,
            $( id = $id:literal, args = [ $( $args:literal ),* $(,)* ]  ),+ $(,)*
      );+ $(;)*
    ) => {
        mod iai_wrappers {
            $(
                #[inline(never)]
                pub fn $before() {
                    let _ = $crate::black_box(super::$before());
                }
            )?
            $(
                #[inline(never)]
                pub fn $after() {
                    let _ = $crate::black_box(super::$after());
                }
            )?
            $(
                #[inline(never)]
                pub fn $setup() {
                    let _ = $crate::black_box(super::$setup());
                }
            )?
            $(
                #[inline(never)]
                pub fn $teardown() {
                    let _ = $crate::black_box(super::$teardown());
                }
            )?
        }

        #[inline(never)]
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.6.1";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--bin-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable benchmark binary

            let mut benchmark = $crate::internal::RunnerBinaryBenchmark::default();
            let mut group = $crate::internal::RunnerBinaryBenchmarkGroup::default();

            $(
                group.sandbox = $sandbox;
            )?

            $(
                let path : &str = $fixtures;
                let mut follow_symlinks : bool = false;
                $(
                    follow_symlinks = $follow_symlinks;
                )?
                group.fixtures = Some($crate::internal::RunnerFixtures {
                    path: path.into(), follow_symlinks
                });
            )?

            $(
                let display : &str = $cmd;
                let command : &str = option_env!(concat!("CARGO_BIN_EXE_", $cmd)).unwrap_or(display);
                let mut opt_arg : Option<$crate::internal::RunnerOptions> = None;
                $(
                    opt_arg = Some($opt.into());
                )?

                let mut env_arg : Vec<String> = vec![];
                $(
                    let envs : Vec<&str> = vec![$($envs),*];
                    env_arg = envs.into_iter().map(|s| s.to_owned()).collect();
                )?

                let mut run_arg : Vec<$crate::internal::RunnerArg> = vec![];
                $(
                    let args : Vec<&str> = vec![$($args),*];
                    let id : &str = $id;
                    let id : Option<String> = Some(id.to_owned());
                    run_arg.push($crate::internal::RunnerArg {
                        id, args: args.into_iter().map(|s| std::ffi::OsString::from(s)).collect()
                    });
                )+
                let run = $crate::internal::RunnerRun {
                    cmd: Some($crate::internal::RunnerCmd {
                        display: display.to_owned(), cmd: command.to_owned()
                    }),
                    args: run_arg,
                    opts: opt_arg,
                    envs: env_arg
                };
                group.benches.push(run);
            )+

            let mut assists : Vec<$crate::internal::RunnerAssistant> = vec![];
            $(
                let mut bench_before = false;
                $(
                    bench_before = $bench_before;
                )?
                assists.push($crate::internal::RunnerAssistant {
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
                assists.push($crate::internal::RunnerAssistant {
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
                assists.push($crate::internal::RunnerAssistant {
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
                assists.push($crate::internal::RunnerAssistant {
                    id: "teardown".to_owned(),
                    name: stringify!($teardown).to_owned(),
                    bench: bench_teardown
                });
            )?

            group.assists = assists;
            benchmark.groups.push(group);

            // Add the callgrind_args first so that arguments from the command line will overwrite
            // those passed to this main macro
            let options : Vec<&str> = vec![$($($options),+)?];

            let mut args : Vec<String> = Vec::with_capacity(options.len() + 10);
            for option in options {
                if option.starts_with("--") {
                    args.push(option.to_owned());
                } else {
                    args.push(format!("--{}", option))
                }
            }

            args.extend(this_args); // The rest of the arguments from the command line
            benchmark.config = $crate::internal::RunnerConfig {
                raw_callgrind_args: args
            };

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
            let mut args_iter = $crate::black_box(std::env::args()).skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                match $crate::black_box(args_iter.next().expect("Expecting a function type")).as_str() {
                    $(
                        "before" => (iai_wrappers::$before)(),
                    )?
                    $(
                        "after" => (iai_wrappers::$after)(),
                    )?
                    $(
                        "setup" => (iai_wrappers::$setup)(),
                    )?
                    $(
                        "teardown" => (iai_wrappers::$teardown)(),
                    )?
                    name => panic!("function '{}' not found in this scope", name)
                }
            } else {
                $crate::black_box(run());
            };
        }
    };
    (
        $( config = $config:expr; $(;)* )?
        binary_benchmark_groups = $( $group:ident ),+ $(,)*
    ) => {
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.6.1";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--bin-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable benchmark binary

            let mut benchmark = $crate::internal::RunnerBinaryBenchmark::default();
            $(
                let mut group = $crate::BinaryBenchmarkGroup::from(
                    $crate::internal::RunnerBinaryBenchmarkGroup {
                        id: Some(stringify!($group).to_owned()),
                        cmd: None,
                        fixtures: None,
                        sandbox: true,
                        benches: Vec::default(),
                        assists: Vec::default(),
                    }
                );
                let (prog, assists) = $group::$group(&mut group);

                let mut group: $crate::internal::RunnerBinaryBenchmarkGroup = group.into();
                group.cmd = prog;
                group.assists = assists;

                benchmark.groups.push(group);
            )+

            let mut config: Option<$crate::internal::RunnerConfig> = None;
            $(
                config = Some($config.into());
            )?
            benchmark.config = if let Some(mut config) = config {
                config.raw_callgrind_args.extend(this_args);
                config
            } else {
                $crate::internal::RunnerConfig {
                    raw_callgrind_args: Vec::from_iter(this_args)
                }
            };

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
            let mut args_iter = $crate::black_box(std::env::args()).skip(1);
            if args_iter
                .next()
                .as_ref()
                .map_or(false, |value| value == "--iai-run")
            {
                match $crate::black_box(args_iter.next().expect("Expecting a function type")).as_str() {
                    $(
                        concat!(stringify!($group), "::", "before") => $group::before(),
                        concat!(stringify!($group), "::", "after") => $group::after(),
                        concat!(stringify!($group), "::", "setup") => $group::setup(),
                        concat!(stringify!($group), "::", "teardown") => $group::teardown(),
                    )+
                    name => panic!("function '{}' not found in this scope", name)
                }
            } else {
                $crate::black_box(run());
            };
        }
    };
    ( callgrind_args = $( $args:literal ),* $(,)*; functions = $( $func_name:ident ),+ $(,)* ) => {
        mod iai_wrappers {
            $(
                pub fn $func_name() {
                    let _ = $crate::black_box(super::$func_name());
                }
            )+
        }

        #[inline(never)]
        fn run(benchmarks: &[&(&'static str, fn())], mut this_args: std::env::Args) {
            let exe =option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.6.1";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--lib-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable

            for bench in benchmarks {
                cmd.arg(format!("--iai-bench={}", bench.0));
            }

            let mut args = Vec::with_capacity(40);
            // Add the callgrind_args first so that arguments from the command line will overwrite
            // those passed to this main macro
            let callgrind_args : Vec<&str> = vec![
                $(
                    $args,
                )*
            ];
            for arg in callgrind_args {
                if arg.starts_with("--") {
                    args.push(arg.to_owned());
                } else {
                    args.push(format!("--{}", arg))
                }
            }

            args.extend(this_args); // The rest of the arguments
            let status = cmd
                .args(args)
                .status()
                .expect("Failed to run benchmarks. \
                    Is iai-callgrind-runner installed and iai-callgrind-runner in your $PATH?. \
                    You can set the environment variable IAI_CALLGRIND_RUNNER to the \
                    absolute path of the iai-callgrind-runner executable.");

            if !status.success() {
                std::process::exit(1);
            }
        }

        fn main() {
            let benchmarks : &[&(&'static str, fn())]= $crate::black_box(&[
                $(
                    &(stringify!($func_name), iai_wrappers::$func_name),
                )+
            ]);

            let mut args_iter = $crate::black_box(std::env::args()).skip(1);
            if args_iter.next().as_ref().map_or(false, |value| value == "--iai-run") {
                let index = $crate::black_box(args_iter
                    .next()
                    .and_then(|arg| arg.parse::<usize>().ok())
                    .expect("Error parsing index"));
                benchmarks[index].1();
            } else {
                run($crate::black_box(benchmarks), $crate::black_box(std::env::args()));
            };
        }
    };

    ( $( $func_name:ident ),+ $(,)* ) => {
        main!(callgrind_args = ; functions = $( $func_name ),+  );
    }
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
/// and options for the sandbox. See also [`crate::BinaryBenchmarkGroup`] for more details.
///
/// The following top-level arguments are accepted:
///
/// ```rust
/// # use iai_callgrind::{binary_benchmark_group, BinaryBenchmarkGroup};
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
        name = $name:ident; $(;)*
        $(before = $before:ident $(,bench = $bench_before:literal)? ; $(;)*)?
        $(after = $after:ident $(,bench = $bench_after:literal)? ; $(;)*)?
        $(setup = $setup:ident $(,bench = $bench_setup:literal)? ; $(;)*)?
        $(teardown = $teardown:ident $(,bench = $bench_teardown:literal)? ; $(;)*)?
        benchmark = |$cmd:expr, $group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        pub mod $name {
            #[inline(never)]
            pub fn before() {
                $(
                    let _ = $crate::black_box(super::$before());
                )?
            }

            #[inline(never)]
            pub fn after() {
                $(
                    let _ = $crate::black_box(super::$after());
                )?
            }

            #[inline(never)]
            pub fn setup() {
                $(
                    let _ = $crate::black_box(super::$setup());
                )?
            }

            #[inline(never)]
            pub fn teardown() {
                $(
                    let _ = $crate::black_box(super::$teardown());
                )?
            }

            #[inline(never)]
            pub fn $name($group: &mut $crate::BinaryBenchmarkGroup) ->
                (Option<$crate::internal::RunnerCmd>, Vec<$crate::internal::RunnerAssistant>)
            {
                let cmd: &str = $cmd;
                let cmd = (!cmd.is_empty()).then(|| $crate::internal::RunnerCmd {
                        display: cmd.to_owned(),
                        cmd: option_env!(concat!("CARGO_BIN_EXE_", $cmd)).unwrap_or(cmd).to_owned()
                    }
                );

                let mut assists: Vec<$crate::internal::RunnerAssistant> = vec![];
                $(
                    let mut bench_before = false;
                    $(
                        bench_before = $bench_before;
                    )?
                    assists.push($crate::internal::RunnerAssistant {
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
                    assists.push($crate::internal::RunnerAssistant {
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
                    assists.push($crate::internal::RunnerAssistant {
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
                    assists.push($crate::internal::RunnerAssistant {
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
        benchmark = |$group:ident: &mut BinaryBenchmarkGroup| $body:expr
    ) => {
        $crate::binary_benchmark_group!(
            name = $name;
            $(before = $before $(,bench = $bench_before)?;)?
            $(after = $after $(,bench = $bench_after)?;)?
            $(setup = $setup $(,bench = $bench_setup)?;)?
            $(teardown = $teardown $(,bench = $bench_teardown)?;)?
            benchmark = |"", $group: &mut BinaryBenchmarkGroup| $body
        );
    };
}
