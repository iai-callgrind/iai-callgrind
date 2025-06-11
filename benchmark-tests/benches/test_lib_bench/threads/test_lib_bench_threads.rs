use std::hint::black_box;
use std::process::Command;

use benchmark_tests::{find_primes_multi_thread, thread_in_thread_with_instrumentation};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Bbv, Callgrind, Dhat, Drd, EntryPoint,
    LibraryBenchmarkConfig, Massif, Memcheck, OutputFormat,
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            "toggle-collect=*::find_primes"
        ]))
)]
#[bench::two(2)]
#[bench::three(3)]
fn bench_find_primes_multi_thread(num_threads: usize) -> Vec<u64> {
    black_box(find_primes_multi_thread(num_threads))
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args([
            "toggle-collect=thread::main",
            "toggle-collect=*::find_primes",
        ]))
)]
#[bench::two(2)]
#[bench::three(3)]
fn bench_thread_in_subprocess(num_threads: usize) {
    Command::new(env!("CARGO_BIN_EXE_thread"))
        .arg(num_threads.to_string())
        .status()
        .unwrap();
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--instr-atstart=no"])
            .entry_point(EntryPoint::None)
        )
)]
fn bench_thread_in_thread() -> Vec<u64> {
    iai_callgrind::client_requests::callgrind::start_instrumentation();
    let result = black_box(thread_in_thread_with_instrumentation());
    iai_callgrind::client_requests::callgrind::stop_instrumentation();
    result
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["instr-atstart=no"])
            .entry_point(EntryPoint::None)
        )
)]
fn bench_thread_in_thread_in_subprocess() {
    iai_callgrind::client_requests::callgrind::start_instrumentation();
    Command::new(env!("CARGO_BIN_EXE_thread"))
        .arg("--thread-in-thread")
        .status()
        .unwrap();
    iai_callgrind::client_requests::callgrind::stop_instrumentation();
}

library_benchmark_group!(
    name = bench_group;
    compare_by_id = true;
    benchmarks =
        bench_find_primes_multi_thread,
        bench_thread_in_subprocess,
        bench_thread_in_thread,
        bench_thread_in_thread_in_subprocess
);

main!(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .truncate_description(None)
            .show_intermediate(true)
        )
        // Helgrind is excluded since an assertion in helgrind itself fails and causes an error.
        // Looks like a bug in valgrind.
        .tool(Dhat::default())
        .tool(Memcheck::default())
        .tool(Drd::default())
        .tool(Massif::default())
        .tool(Bbv::default());
    library_benchmark_groups = bench_group
);
