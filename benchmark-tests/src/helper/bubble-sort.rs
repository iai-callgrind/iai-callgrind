use benchmark_tests::{bubble_sort, setup_worst_case_array};

fn main() {
    let mut iter = std::env::args().skip(1);
    let start = iter.next().unwrap().parse().unwrap();
    let sorted = bubble_sort(setup_worst_case_array(start));
    println!("{sorted:?}");
}
