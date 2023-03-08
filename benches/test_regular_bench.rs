use iai_callgrind::{black_box, main};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// Don't forget the `#[inline(never)]`
#[inline(never)]
fn iai_benchmark_short() -> u64 {
    fibonacci(black_box(10))
}

#[inline(never)]
fn iai_benchmark_long() -> u64 {
    fibonacci(black_box(30))
}

main!(iai_benchmark_short, iai_benchmark_long);
