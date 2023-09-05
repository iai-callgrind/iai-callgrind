#![allow(clippy::unit_arg)]

use benchmark_tests::print_env;
use iai_callgrind::{
    black_box, library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
};

#[library_benchmark]
#[bench::single(&["HOME"])]
fn bench_print_env_single(args: &[&str]) {
    black_box(print_env(args))
}

#[library_benchmark]
#[bench::multiple(&["HOME", "USER"])]
fn bench_print_env_multiple(args: &[&str]) {
    black_box(print_env(args))
}

#[library_benchmark]
#[bench::single(&["FOO=BAR"])]
fn bench_print_env_custom_single(args: &[&str]) {
    black_box(print_env(args))
}

#[library_benchmark]
#[bench::multiple(&["FOO=BAR", "BAR=BAZ"])]
fn bench_print_env_custom_multiple(args: &[&str]) {
    black_box(print_env(args))
}

library_benchmark_group!(
    name = pass_through_single;
    config = LibraryBenchmarkConfig::default().clear_env(true).pass_through_env("HOME");
    benchmarks = bench_print_env_single
);

library_benchmark_group!(
    name = pass_through_multiple;
    config = LibraryBenchmarkConfig::default().clear_env(true).pass_through_envs(["HOME", "USER"]);
    benchmarks = bench_print_env_multiple
);

library_benchmark_group!(
    name = custom_single;
    config = LibraryBenchmarkConfig::default().clear_env(true).env("FOO", "BAR");
    benchmarks = bench_print_env_custom_single
);

library_benchmark_group!(
    name = custom_multiple;
    config = LibraryBenchmarkConfig::default().clear_env(true).envs([("FOO", "BAR"), ("BAR","BAZ")]);
    benchmarks = bench_print_env_custom_multiple
);

main!(
    library_benchmark_groups = pass_through_single,
    pass_through_multiple,
    custom_single,
    custom_multiple
);
