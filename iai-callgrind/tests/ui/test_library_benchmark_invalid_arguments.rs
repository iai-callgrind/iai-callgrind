use iai_callgrind::library_benchmark;

#[library_benchmark(wrong = LibraryBenchmarkConfig::default())]
fn bench1() {}

#[library_benchmark(config = "wrong")]
fn bench2() {}

#[library_benchmark(config = )]
fn bench3() {}

fn main() {}
