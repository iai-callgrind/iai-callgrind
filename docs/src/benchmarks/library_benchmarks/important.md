# Important default behaviour

The environment variables are cleared before running a library benchmark. Have a
look into the [Configuration](./configuration.md) section if you need to change that
behavior.

Per default, the benchmarks run with cache simulation switched on. This adds
additional run time costs. If you don't need the cache metrics and estimation of
cycles, yan can easily switch cache simulation off with

```rust
# extern crate iai_callgrind;
use iai_callgrind::LibraryBenchmarkConfig;

LibraryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
```

For example to switch off cache simulation for all benchmarks in the same file:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn fibonacci(a: u64) -> u64 { a } }
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig
};
use std::hint::black_box;

#[library_benchmark]
fn bench_fibonacci() -> u64 {
    black_box(my_lib::fibonacci(10))
}

library_benchmark_group!(name = fibonacci_group; benchmarks = bench_fibonacci);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
    library_benchmark_groups = fibonacci_group
);
# }
```

If you're new to Iai-Callgrind and don't know what the above means, don't panic.
Jump to [Quickstart](./quickstart.md) and read through the first few chapters,
and you're ready to benchmark with Iai-Callgrind like a pro.
