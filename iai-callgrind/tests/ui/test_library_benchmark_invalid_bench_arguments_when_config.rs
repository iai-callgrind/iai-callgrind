use iai_callgrind::library_benchmark;

#[library_benchmark]
#[bench::id(config = "some")]
pub fn bench0() {}

#[library_benchmark]
#[bench::id(wrong = LibraryBenchmarkConfig::default())]
pub fn bench1() {}

#[library_benchmark]
#[bench::id(config = )]
pub fn bench2() {}

fn main() {}
