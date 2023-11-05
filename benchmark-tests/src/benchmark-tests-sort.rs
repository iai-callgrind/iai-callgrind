use benchmark_tests::bubble_sort_allocate;

fn main() {
    let mut iter = std::env::args().skip(1);
    let start = iter.next().unwrap_or("4000".to_owned()).parse().unwrap();
    let sum = iter.next().unwrap_or("2000".to_owned()).parse().unwrap();

    let sum = bubble_sort_allocate(start, sum);
    println!("{sum}");
}
