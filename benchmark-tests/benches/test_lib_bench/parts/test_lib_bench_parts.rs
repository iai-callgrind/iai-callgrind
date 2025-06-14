use std::hint::black_box;
use std::process::{Command, ExitStatus};

use benchmark_tests::{find_primes, find_primes_multi_thread_with_instrumentation};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Callgrind, EntryPoint,
    LibraryBenchmarkConfig, OutputFormat,
};

#[library_benchmark]
#[bench::dump_every_bb(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-every-bb=100000"]))
)]
#[bench::dump_before(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-before=*::find_primes"]))
)]
#[bench::dump_after(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-after=*::find_primes"]))
)]
fn bench_no_thread() -> Vec<u64> {
    black_box(find_primes(0, 20000))
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--instr-atstart=no"])
            .entry_point(EntryPoint::None)
        )
)]
#[bench::dump_every_bb(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-every-bb=100000"]))
)]
#[bench::dump_before(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-before=*::find_primes"]))
)]
#[bench::dump_after(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-after=*::find_primes"]))
)]
fn bench_multiple_threads() -> Vec<u64> {
    iai_callgrind::client_requests::callgrind::start_instrumentation();
    let result = black_box(find_primes_multi_thread_with_instrumentation(2));
    iai_callgrind::client_requests::callgrind::stop_instrumentation();
    result
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--instr-atstart=no"])
            .entry_point(EntryPoint::None)
        )
)]
#[bench::dump_every_bb(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-every-bb=100000"]))
)]
#[bench::dump_before(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-before=*::find_primes"]))
)]
#[bench::dump_after(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--dump-after=*::find_primes"]))
)]
fn bench_multiple_threads_in_subprocess() -> ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_thread"))
        .arg("--thread-in-thread")
        .status()
        .unwrap()
}

library_benchmark_group!(
    name = my_group;
    benchmarks =
        bench_no_thread,
        bench_multiple_threads,
        bench_multiple_threads_in_subprocess
);

main!(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .show_intermediate(true)
        );
    library_benchmark_groups = my_group);
