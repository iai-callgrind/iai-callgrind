<!-- markdownlint-disable MD041 MD033 -->

# Important default behaviour

As in library benchmarks, the environment variables are cleared before running a
binary benchmark. Have a look at the [Configuration](./configuration.md) section
if you want to change this behavior. Iai-Callgrind sometimes deviates from the
valgrind defaults which are:

| Iai-Callgrind | Valgrind (v3.23) |
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
# extern crate iai_callgrind;
use iai_callgrind::BinaryBenchmarkConfig;

BinaryBenchmarkConfig::default().callgrind_args(["--cache-sim=no"]);
```

To switch off cache simulation for all benchmarks in the same file:

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

<!-- TODO: ALSO IN library_benchmarks -->
<!-- TODO: Update all example outputs with the summary -->
<!-- TODO: Talk about --nosummary in cli_and_env -->
Per default, Iai-Callgrind reports the cache hits and an estimation of cpu
cycles:

<pre><code class="hljs"><span style="color:#0A0">test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
<span style="color:#555">  </span>Instructions:                        <b>1734</b>|1734                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>2359</b>|2359                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Total read+write:                    <b>2362</b>|2362                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Estimated Cycles:                    <b>2464</b>|2464                 (<span style="color:#555">No change</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

If you prefer cache misses over cache hits or just want both metrics displayed
you can fully customize the [callgrind output
format](../library_benchmarks/configuration/output_format.md).
