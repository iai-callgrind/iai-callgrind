use iai_callgrind::library_benchmark;

// missing argument of the benchmark
#[library_benchmark]
#[bench::id(42)]
fn bench1() {}

// missing argument of the bench attribute
#[library_benchmark]
#[bench::id()]
fn bench2(my: i32) {}

// too many arguments of the bench attribute
#[library_benchmark]
#[bench::id(42, 8)]
fn bench3(my: i32) {}

// incorrect argument type
#[library_benchmark]
#[bench::id("hello")]
fn bench4(my: u8) {}

// incorrect return type
#[library_benchmark]
#[bench::id(42)]
fn bench5(my: u8) -> String {
    my
}

#[library_benchmark]
pub fn bench6() {}

#[library_benchmark]
#[bench::id()]
pub fn bench7() {}

fn main() {
    // check that bench5 isn't public
    bench5::bench5();
    // check that bench6 isn't public anymore
    bench6::bench6();
    // check that bench7 isn't public anymore
    bench7::bench7();
}
