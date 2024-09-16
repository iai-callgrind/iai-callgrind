use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Command,
};

#[binary_benchmark]
#[bench::parse_output()]
fn run() -> Command {
    let path = env!("CARGO_BIN_EXE_client-requests");
    Command::new(path).build()
}

binary_benchmark_group!(
    name = client_requests;
    config = BinaryBenchmarkConfig::default()
        .raw_callgrind_args([
            "--instr-atstart=no"
    ]);
    benchmarks = run,
);

main!(binary_benchmark_groups = client_requests);