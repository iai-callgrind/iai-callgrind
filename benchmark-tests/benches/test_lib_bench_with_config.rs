use benchmark_tests::bubble_sort;
use iai_callgrind::{black_box, main, LibraryBenchmarkConfig};

#[export_name = "helper::setup_array"]
#[inline(never)]
fn setup_array(start: i32) -> Vec<i32> {
    (0..start).rev().collect()
}

#[inline(never)]
fn bench_bubble_bad() -> Vec<i32> {
    let array = black_box(vec![6, 5, 4, 3, 2, 1]);
    bubble_sort(array)
}

#[inline(never)]
fn bench_bubble_with_expensive_setup() -> Vec<i32> {
    bubble_sort(black_box(setup_array(4000)))
}

main!(
    config = LibraryBenchmarkConfig::default().raw_callgrind_args(["toggle-collect=helper::*"]);
    functions = bench_bubble_bad, bench_bubble_with_expensive_setup);
