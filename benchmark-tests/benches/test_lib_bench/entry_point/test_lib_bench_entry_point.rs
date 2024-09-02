use std::hint::black_box;

use benchmark_tests::assert::Assert;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EntryPoint, EventKind, LibraryBenchmarkConfig,
};
use iai_callgrind_runner::runner::callgrind::hashmap_parser::SourcePath;
use iai_callgrind_runner::runner::summary::BenchmarkSummary;

#[inline(never)]
fn nested() -> u64 {
    benchmark_tests::fibonacci(10)
}

#[inline(never)]
fn some_func() -> u64 {
    nested()
}

#[library_benchmark]
#[bench::none(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::None),
)]
#[bench::default(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::Default)
)]
#[bench::nested(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::from("test_lib_bench_entry_point::nested"))
)]
fn bench_lib() -> u64 {
    black_box(some_func())
}

// We need to check some gory details in the group teardown to see if the entry point is being
// applied correctly.
fn assert_none() {
    let assert = Assert::new(module_path!(), "my_group", "bench_lib", "none").unwrap();
    assert
        .summary(|b| {
            let new_ir = b.callgrind_summary.unwrap().summaries[0]
                .events
                .diff_by_kind(&EventKind::Ir)
                .unwrap()
                .new
                .unwrap();
            new_ir > 400000
        })
        .unwrap();
}

fn assert_default() {
    let check_summary = |b: BenchmarkSummary| {
        let new_ir = b.callgrind_summary.unwrap().summaries[0]
            .events
            .diff_by_kind(&EventKind::Ir)
            .unwrap()
            .new
            .unwrap();
        new_ir < 3000
    };

    let assert = Assert::new(module_path!(), "my_group", "bench_lib", "default").unwrap();
    assert.summary(check_summary).unwrap();
    assert
        .callgrind_map(|m| {
            let main_costs = m.map
                .iter()
                .find_map(|(k, v)| (k.func == "main").then(|| v.costs.clone()))
                .unwrap();

            let benchmark_function_costs = m.map
                    .iter()
                    .find_map(|(k, v)| {
                        (
                            k.func == "test_lib_bench_entry_point::bench_lib::__iai_callgrind_wrapper_mod::bench_lib" &&
                            k.file == Some(SourcePath::Relative(file!().into()))
                        )
                        .then(|| v.costs.clone())
                    })
                    .unwrap();
            main_costs == benchmark_function_costs
        })
        .unwrap();
}

fn assert_nested() {
    let check_summary = |b: BenchmarkSummary| {
        let new_ir = b.callgrind_summary.unwrap().summaries[0]
            .events
            .diff_by_kind(&EventKind::Ir)
            .unwrap()
            .new
            .unwrap();
        new_ir < 3000
    };

    let assert = Assert::new(module_path!(), "my_group", "bench_lib", "nested").unwrap();
    assert.summary(check_summary).unwrap();
    assert
        .callgrind_map(|m| {
            let main_costs = m
                .map
                .iter()
                .find_map(|(k, v)| (k.func == "main").then(|| v.costs.clone()))
                .unwrap();
            let nested_function_costs = m
                .map
                .iter()
                .find_map(|(k, v)| {
                    (k.func == "test_lib_bench_entry_point::nested"
                        && k.file == Some(SourcePath::Relative(file!().into())))
                    .then(|| v.costs.clone())
                })
                .unwrap();
            main_costs == nested_function_costs
        })
        .unwrap();
}

fn assert_benchmarks() {
    assert_none();
    assert_default();
    assert_nested();
}

library_benchmark_group!(name = my_group; teardown = assert_benchmarks(); benchmarks = bench_lib);
main!(library_benchmark_groups = my_group);
