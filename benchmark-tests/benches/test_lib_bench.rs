use iai_callgrind::{black_box, main};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[inline(never)]
fn bench_empty() {}

#[inline(never)]
fn bench_fibonacci() -> u64 {
    fibonacci(black_box(10))
}

#[inline(never)]
fn bench_fibonacci_long() -> u64 {
    fibonacci(black_box(30))
}

main!(bench_empty, bench_fibonacci, bench_fibonacci_long);
