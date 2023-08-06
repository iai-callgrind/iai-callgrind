fn main() {
    println!("{}", fibonacci(30));
}

#[inline(never)]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
