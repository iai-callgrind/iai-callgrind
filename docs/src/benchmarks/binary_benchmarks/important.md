<!-- markdownlint-disable MD041 MD033 -->

# Important default behaviour

As in library benchmarks, the environment variables are cleared before running a
binary benchmark. Have a look at the [Configuration](./configuration.md) section
if you want to change this behavior. Gungraun sometimes deviates from the
valgrind defaults which are:

| Gungraun | Valgrind (v3.23) |
| ------------- | -------- |
| `--trace-children=yes` | `--trace-children=no` |
| `--fair-sched=try` | `--fair-sched=no` |
| `--separate-threads=yes` | `--separate-threads=no` |
| `--cache-sim=yes` | `--cache-sim=no` |

As show in the table above, the benchmarks run with cache simulation switched
on. This adds run time for each benchmark. If you don't need the cache metrics
and estimation of cycles, you can easily switch cache simulation off for example
with

```rust
# extern crate gungraun;
use gungraun::{BinaryBenchmarkConfig, Callgrind};

BinaryBenchmarkConfig::default().tool(Callgrind::with_args(["--cache-sim=no"]));
```

To switch off cache simulation for all benchmarks in the same file:

```rust
# extern crate gungraun;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use gungraun::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig,
    Callgrind
};

#[binary_benchmark]
fn bench_binary() -> gungraun::Command {
    gungraun::Command::new(env!("CARGO_BIN_EXE_my-foo"))
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(
    config = BinaryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--cache-sim=no"]));
    binary_benchmark_groups = my_group
);
# }
```
