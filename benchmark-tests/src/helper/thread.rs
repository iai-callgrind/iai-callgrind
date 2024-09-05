fn is_prime(num: u64) -> bool {
    if num <= 1 {
        return false;
    }

    for i in 2..=(num as f64).sqrt() as u64 {
        if num % i == 0 {
            return false;
        }
    }

    true
}

fn find_primes(low: u64, high: u64) -> Vec<u64> {
    (low..=high).filter(|n| is_prime(*n)).collect()
}

fn main() {
    let mut args_iter = std::env::args().skip(1);

    let num = args_iter.next().map_or(0, |a| a.parse::<usize>().unwrap());

    let mut handles = vec![];
    let mut low = 0;
    for _ in 0..num {
        let handle = std::thread::spawn(move || find_primes(low, low + 10000));
        handles.push(handle);

        low += 10000;
    }

    let mut primes = vec![];
    for handle in handles {
        let result = handle.join();
        primes.extend(result.unwrap())
    }

    println!(
        "Number of primes found in the range 0 to {low}: {}",
        primes.len()
    );
}
