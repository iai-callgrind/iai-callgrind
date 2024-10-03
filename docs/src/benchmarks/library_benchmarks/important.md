# Important default behaviour

The environment variables are cleared before running a library benchmark. Have a
look into the [Configuration](./configuration.md) section if you need to change
that behavior. Iai-Callgrind sometimes deviates from the valgrind defaults which
are:

| Iai-Callgrind | Valgrind (v3.23) |
| ------------- | ---------------- |
| `--trace-children=yes` | `--trace-children=no` |
| `--fair-sched=try` | `--fair-sched=no` |
| `--separate-threads=yes` | `--separate-threads=no` |
| `--cache-sim=yes` | `--cache-sim=no` |

The thread and subprocess specific valgrind options enable tracing threads and
subprocesses basically but there's usually some additional configuration
necessary to trace the metrics of threads and subprocesses.

As show in the table above, the benchmarks run with cache simulation switched
on. This adds run time. If you don't need the cache metrics and estimation of
cycles, you can easily switch cache simulation off for example with:

```rust
# extern crate iai_callgrind;
use iai_callgrind::LibraryBenchmarkConfig;

LibraryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
```

To switch off cache simulation for all benchmarks in the same file:

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
