use iai_callgrind::library_benchmark;

// missing id
#[library_benchmark]
#[bench]
fn bench1() {}

#[library_benchmark]
#[bench::missing_parentheses]
fn bench2() {}

#[library_benchmark]
#[bench::same()]
#[bench::same()]
fn bench3() {}

fn main() {}
