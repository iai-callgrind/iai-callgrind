use std::collections::HashMap;

use benchmark_tests::find_primes;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, EntryPoint, FlamegraphConfig,
    LibraryBenchmarkConfig, Tool, ValgrindTool,
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::None)
        .raw_callgrind_args(["--fair-sched=yes"])
        .tool(Tool::new(ValgrindTool::DHAT)
            .args(["--trace-children=yes"])
            .outfile_modifier("%p"))
)]
#[bench::two(2)]
#[bench::three(3)]
fn bench_library(num: u64) {
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

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .entry_point(EntryPoint::None)
        .raw_callgrind_args(["--fair-sched=yes"])
        .tool(Tool::new(ValgrindTool::DHAT)
            .args(["--trace-children=yes"])
            .outfile_modifier("%p"))
)]
#[bench::two(3)]
#[bench::three(3)]
fn bench_library_compare(num: u64) {
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

fn get_complex_map() -> HashMap<(String, String, String), u64> {
    let mut map = HashMap::new();
    map.insert(
        ("hello".to_owned(), "world".to_owned(), "and".to_owned()),
        10,
    );
    map
}

fn get_simple_map() -> HashMap<u64, u64> {
    let mut map = HashMap::new();
    map.insert(0, 10);
    map
}

fn insert_with_entry<T>(mut map: HashMap<T, u64>, key: &T) -> u64
where
    T: Clone + Eq + std::hash::Hash,
{
    *map.entry(key.clone()).and_modify(|v| *v += 10).or_insert(0)
}

fn insert_normal<T>(mut map: HashMap<T, u64>, key: &T) -> u64
where
    T: Clone + Eq + std::hash::Hash,
{
    if let Some(value) = map.get_mut(key) {
        *value += 10;
    } else {
        map.insert(key.clone(), 0);
    }
    0
}

#[library_benchmark]
#[bench::not_present_complex(("HELLO".to_owned(), "world".to_owned(), "and".to_owned()), get_complex_map())]
#[bench::present_complex(("hello".to_owned(), "world".to_owned(), "and".to_owned()), get_complex_map())]
#[bench::not_present_simple(1, get_simple_map())]
#[bench::present_simple(0, get_simple_map())]
fn with_entry<T>(key: T, map: HashMap<T, u64>) -> u64
where
    T: Clone + Eq + std::hash::Hash,
{
    std::hint::black_box(insert_with_entry(map, &key))
}

#[library_benchmark]
#[bench::not_present_complex(("HELLO".to_owned(), "world".to_owned(), "nope".to_owned()), get_complex_map())]
#[bench::present_complex(("hello".to_owned(), "world".to_owned(), "and".to_owned()), get_complex_map())]
#[bench::not_present_simple(1, get_simple_map())]
#[bench::present_simple(0, get_simple_map())]
fn normal<T>(key: T, map: HashMap<T, u64>) -> u64
where
    T: Clone + Eq + std::hash::Hash,
{
    std::hint::black_box(insert_normal(map, &key))
}

library_benchmark_group!(
    name = my_group;
    config = LibraryBenchmarkConfig::default()
        .truncate_description(None)
        .flamegraph(FlamegraphConfig::default());
    compare_by_id = true;
    benchmarks = bench_library, bench_library_compare, normal, with_entry
);
main!(library_benchmark_groups = my_group);
