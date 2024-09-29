# Important default behaviour

As in library benchmarks, the environment variables are cleared before running a
binary benchmark. Have a look at the [Configuration](./configuration.md) section
if you want to change this behavior.

Per default, the benchmarks run with cache simulation switched on. This adds
additional run time costs. If you don't need the cache metrics and estimation of
cycles, yan can easily switch cache simulation off with

```rust
# extern crate iai_callgrind;
use iai_callgrind::BinaryBenchmarkConfig;

BinaryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
```

For example to switch off cache simulation for all benchmarks in the same file:

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig
};

#[binary_benchmark]
fn bench_binary() -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(
    config = BinaryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
    binary_benchmark_groups = my_group
);
# }
```

If you're new to Iai-Callgrind and don't know what the above means, don't panic.
Jump to [Quickstart](./quickstart.md) and read through the first few chapters,
and you're ready to benchmark with Iai-Callgrind like a pro.
