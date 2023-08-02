//! Contains macros which together define a benchmark harness that can be used in place of the
//! standard benchmark harness. This allows the user to run Iai benchmarks with `cargo bench`.

/// Macro which expands to a benchmark harness.
///
/// This macro has two forms:
///
/// ```ignore
/// main!(func1, func2)
/// ```
///
/// or
///
/// ```ignore
/// main!(
///     callgrind_args = "--arg-with-flags=yes", "arg-without-flags=is_ok_too"
///     functions = func1, func2
/// )
/// ```
///
/// Using Iai-callgrind requires disabling the benchmark harness generated automatically by rustc.
/// This can be done like so:
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
/// `my_bench` must be a rust file inside the 'benches' directory, like so:
///
/// `benches/my_bench.rs`
///
/// Since we've disabled the default benchmark harness, we need to add our own:
///
/// ```ignore
/// use iai_callgrind::main;
///
/// // `#[inline(never)]` is important! Without it there won't be any metrics
/// #[inline(never)]
/// fn bench_method1() {
/// }
///
/// #[inline(never)]
/// fn bench_method2() {
/// }
///
/// main!(bench_method1, bench_method2);
/// ```
///
/// The `iai_callgrind::main` macro expands to a `main` function which runs all of the benchmarks.
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
            $(, envs = [ $( $envs:literal ),* $(,)* ] )?
            $(, opts = $opt:expr )? ,
            $( args = [ $( $args:literal ),* $(,)* ]  ),+ $(,)*
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
        fn to_arg(vec: &[&str]) -> String {
            if let Some((first, remainder)) = vec.split_first() {
                let sep = ",";
                remainder.iter().fold(format!("'{}'", first), |mut a, b| {
                    a.push_str(sep);
                    a.push('\'');
                    a.push_str(b);
                    a.push('\'');
                    a
                })
            } else {
                String::new()
            }
        }

        #[inline(never)]
        fn run() {
            let mut this_args = std::env::args();
            let exe = option_env!("IAI_CALLGRIND_RUNNER")
                .unwrap_or_else(|| option_env!("CARGO_BIN_EXE_iai-callgrind-runner").unwrap_or("iai-callgrind-runner"));

            let library_version = "0.4.0";

            let mut cmd = std::process::Command::new(exe);

            cmd.arg(library_version);
            cmd.arg("--bin-bench");
            cmd.arg(env!("CARGO_MANIFEST_DIR"));
            cmd.arg(file!());
            cmd.arg(module_path!());
            cmd.arg(this_args.next().unwrap()); // The executable benchmark binary

            $(
                let sandbox : bool = $sandbox;
                cmd.arg(format!("--sandbox='{}'", sandbox));
            )?

            $(
                let fixtures : &str = $fixtures;
                let mut follow_symlinks : bool = false;
                $(
                    follow_symlinks = $follow_symlinks;
                )?
                if follow_symlinks {
                    cmd.arg(format!("--fixtures='{}','follow_symlinks=true'", fixtures));
                } else {
                    cmd.arg(format!("--fixtures='{}'", fixtures));
                }
            )?

            use std::fmt::Write;
            use std::path::PathBuf;
            use std::ffi::OsString;
            use $crate::{Options, OptionsParser};

            $(
                let command : &str = option_env!(concat!("CARGO_BIN_EXE_", $cmd)).unwrap_or($cmd);
                $(
                    let args : Vec<&str> = vec![$($args),*];
                    if args.is_empty() {
                        cmd.arg(format!("--run='{}'", command));
                    } else {
                        cmd.arg(format!("--run='{}',{}", command, to_arg(&args)));
                    }
                )+
                $(
                    let envs : Vec<&str> = vec![$($envs),*];
                    if !envs.is_empty() {
                        cmd.arg(format!("--run-envs={}", to_arg(&envs)));
                    }
                )?
                $(
                    let opt_arg = OptionsParser::new($opt).into_arg();
                    if !opt_arg.is_empty() {
                        let mut arg = OsString::new();
                        arg.push("--run-opts=");
                        arg.push(opt_arg);
                        cmd.arg(arg);
                    }
                )?
            )+

            $(
                let mut bench_before = false;
                $(
                    bench_before = $bench_before;
                )?
                if bench_before {
                    cmd.arg(format!("--bench-before={}", stringify!($before)));
                } else {
                    cmd.arg(format!("--before={}", stringify!($before)));
                }
            )?
            $(
                let mut bench_after = false;
                $(
                    bench_after = $bench_after;
                )?
                if bench_after {
                    cmd.arg(format!("--bench-after={}", stringify!($after)));
                } else {
                    cmd.arg(format!("--after={}", stringify!($after)));
                }
            )?
            $(
                let mut bench_setup = false;
                $(
                    bench_setup = $bench_setup;
                )?
                if bench_setup {
                    cmd.arg(format!("--bench-setup={}", stringify!($setup)));
                } else {
                    cmd.arg(format!("--setup={}", stringify!($setup)));
                }
            )?
            $(
                let mut bench_teardown = false;
                $(
                    bench_teardown = $bench_teardown;
                )?
                if bench_teardown {
                    cmd.arg(format!("--bench-teardown={}", stringify!($teardown)));
                } else {
                    cmd.arg(format!("--teardown={}", stringify!($teardown)));
                }
            )?


            // Add the callgrind_args first so that arguments from the command line will overwrite
            // those passed to this main macro
            let options : Vec<&str> = vec![$($($options),+)?];

            let mut args : Vec<String> = Vec::with_capacity(10);
            for option in options {
                if option.starts_with("--") {
                    args.push(option.to_owned());
                } else {
                    args.push(format!("--{}", option))
                }
            }

            args.extend(this_args); // The rest of the arguments

            let status = cmd
                .args(args)
                .status()
                .expect("Failed to run benchmarks. \
                    Is iai-callgrind-runner installed and iai-callgrind-runner in your $PATH?. \
                    You can also set the environment variable IAI_CALLGRIND_RUNNER to the \
                    absolute path of the iai-callgrind-runner executable.");

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

            let library_version = "0.4.0";

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
