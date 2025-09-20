use std::hint::black_box;

use benchmark_tests::{bubble_sort, setup_best_case_array, setup_worst_case_array};
use gungraun::{
    library_benchmark, library_benchmark_group, main, Dhat, EntryPoint, LibraryBenchmarkConfig,
    ValgrindTool,
};

#[inline(never)]
fn custom_setup(start: i32) -> Vec<i32> {
    setup_best_case_array(start);
    setup_worst_case_array(start)
}

#[inline(never)]
fn teardown(mut data: Vec<i32>) {
    let other = std::mem::take(&mut data);
    drop(data);
    drop(other);
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .frames(["*::custom_setup"])
        )
)]
#[bench::with_entry_point(args = (5), setup = custom_setup, teardown = teardown)]
#[bench::without_entry_point(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .entry_point(EntryPoint::None)
        ),
    setup = custom_setup,
    teardown = teardown
)]
fn heap(data: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(data))
}

#[library_benchmark]
#[bench::with_entry_point(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::with_args(["--mode=copy"])),
    setup = custom_setup,
)]
#[bench::without_entry_point(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::with_args(["--mode=copy"])
            .entry_point(EntryPoint::None)
        ),
    setup = custom_setup,
)]
fn copy(mut src: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
    let mut dst: Vec<i32> = Vec::with_capacity(src.len());
    let src_len = src.len();

    unsafe {
        src.set_len(0);

        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), src_len);
        dst.set_len(src_len);
    }

    (src, dst)
}

#[library_benchmark]
#[bench::with_entry_point(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::with_args(["--mode=ad-hoc"])),
    setup = setup_worst_case_array
)]
#[bench::without_entry_point(
    args = (5),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::with_args(["--mode=ad-hoc"])
            .entry_point(EntryPoint::None)
        ),
    setup = setup_worst_case_array
)]
fn ad_hoc(data: Vec<i32>) -> Vec<i32> {
    gungraun::client_requests::dhat::ad_hoc_event(15);
    black_box(bubble_sort(data))
}

#[library_benchmark]
#[bench::five(5)]
fn alloc_in_func(start: i32) -> Vec<i32> {
    setup_worst_case_array(start)
}

library_benchmark_group!(name = my_group; benchmarks = heap, copy, ad_hoc, alloc_in_func);
main!(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DHAT);
    library_benchmark_groups = my_group
);
