# Configure the exit code of the Command

Usually, if a `Command` exits with a non-zero exit code, the whole benchmark run
fails and stops. If the exit code of the benchmarked `Command` is to be expected
different from `0`, the expected exit code can be set in
`BinaryBenchmarkConfig::exit_with` or `Command::exit_with`:

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
     binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, ExitWith
};

#[binary_benchmark]
// Here, we set the expected exit code of `my-foo` to 2
#[bench::exit_with_2(
    config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Code(2))
)]
// Here, we don't know the exact exit code but know it is different from 0 (=success)
#[bench::exit_with_failure(
    config = BinaryBenchmarkConfig::default().exit_with(ExitWith::Failure)
)]
fn bench_binary() -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```
