use benchmark_tests::bubble_sort_allocate;

fn main() {
    let sum = bubble_sort_allocate(4000, 2000);
    println!("{sum}");
}
