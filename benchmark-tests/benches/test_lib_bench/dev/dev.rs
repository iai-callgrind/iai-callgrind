use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

fn print_to_stderr(value: u64) {
    eprintln!("Error output during teardown: {value}");
}

fn add_10_and_print(value: u64) -> u64 {
    let value = value + 10;
    println!("Output to stdout: {value}");

    value
}

#[library_benchmark]
#[bench::some_id(args = (10), teardown = print_to_stderr)]
fn bench_library(value: u64) -> u64 {
    black_box(add_10_and_print(value))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);
main!(library_benchmark_groups = my_group);
