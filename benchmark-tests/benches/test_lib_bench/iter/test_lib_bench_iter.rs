use std::hint::black_box;

use benchmark_tests::{bubble_sort, fibonacci, setup_worst_case_array};
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, Dhat, LibraryBenchmarkConfig,
};

// TODO: TEST STANDALONE with and without setup and/or teardown
// TODO: UI test with wild card `_` in function signature

#[library_benchmark]
#[benches::some_id(iter = vec![(1, 2), (2, 3)])]
fn bench_me((num, _): (usize, usize)) -> u64 {
    black_box(fibonacci(num as u64))
}

#[inline(never)]
fn setup(num: u64) -> u64 {
    num + 1
}

#[inline(never)]
fn allocate_in_setup(inputs: fn() -> Vec<i32>) -> Vec<i32> {
    black_box(inputs())
}

fn teardown(num: u64) -> Result<u64, String> {
    if num < 2 {
        Ok(num)
    } else {
        Err("Something".to_owned())
    }
}

#[library_benchmark]
#[benches::single(iter = vec![1, 2])]
#[benches::with_setup(
    iter = vec![1, 2],
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .frames(["*::setup"])
        ),
    setup = setup
)]
#[benches::with_teardown(iter = vec![1, 2], teardown = teardown)]
#[benches::with_setup_and_teardown(
    iter = vec![1, 2],
    setup = setup,
    teardown = teardown
)]
#[benches::option(iter = Some(1))]
#[benches::range(iter = 1..=5)]
#[benches::iter(iter = vec![1, 2].into_iter().map(|n| n + 10))]
fn bench_single(num: u64) -> u64 {
    black_box(fibonacci(num))
}

#[library_benchmark]
#[benches::with_setup(
    iter = vec![1, 2],
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .frames(["*::setup_worst_case_array"])
        ),
    setup = setup_worst_case_array
)]
#[benches::with_alloc_in_setup(
    iter = vec![|| vec![2, 1], || vec![1]],
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .frames(["*::allocate_in_setup"])
        ),
    setup = allocate_in_setup
)]
fn bench_allocation(inputs: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(inputs))
}

#[library_benchmark]
#[benches::single(iter = vec![1u32, 2u32])]
fn bench_generic<T>(num: T) -> u64
where
    T: Into<u64>,
{
    black_box(fibonacci(num.into()))
}

library_benchmark_group!(
    name = my_group;
    benchmarks =
        bench_me,
        bench_single,
        bench_allocation,
        bench_generic
);
main!(library_benchmark_groups = my_group);
