<!-- markdownlint-disable MD041 MD033 -->

# Important default behaviour

The environment variables are cleared before running a library benchmark. Have a
look into the [Configuration](./configuration.md) section if you need to change
that behavior. Gungraun sometimes deviates from the valgrind defaults which
are:

| Gungraun | Valgrind (v3.23) |
| ------------- | ---------------- |
| `--trace-children=yes` | `--trace-children=no` |
| `--fair-sched=try` | `--fair-sched=no` |
| `--separate-threads=yes` | `--separate-threads=no` |
| `--cache-sim=yes` | `--cache-sim=no` |

The thread and subprocess specific valgrind options enable tracing threads and
subprocesses basically but there's usually some additional configuration
necessary to [trace the metrics of threads and
subprocesses](./threads_and_subprocesses.md).

As show in the table above, the benchmarks run with cache simulation switched
on. This adds run time. If you don't need the cache metrics and estimation of
cycles, you can easily switch cache simulation off for example with:

```rust
# extern crate iai_callgrind;
use iai_callgrind::{LibraryBenchmarkConfig, Callgrind};

LibraryBenchmarkConfig::default().tool(Callgrind::with_args(["--cache-sim=no"]));
```

To switch off cache simulation for all benchmarks in the same file:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn fibonacci(a: u64) -> u64 { a } }
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    Callgrind
};
use std::hint::black_box;

#[library_benchmark]
fn bench_fibonacci() -> u64 {
    black_box(my_lib::fibonacci(10))
}

library_benchmark_group!(name = fibonacci_group; benchmarks = bench_fibonacci);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--cache-sim=no"]));
    library_benchmark_groups = fibonacci_group
);
# }
```

Gungraun reports the cache hits and an estimation of cpu cycles:

<pre><code class="hljs"><span style="color:#0A0">test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
<span style="color:#555">  </span>Instructions:                        <b>1734</b>|1734                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>2359</b>|2359                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>LL Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Total read+write:                    <b>2362</b>|2362                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Estimated Cycles:                    <b>2464</b>|2464                 (<span style="color:#555">No change</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

If you prefer cache misses over cache hits or just want both metrics displayed
you can fully customize the [callgrind output
format](../library_benchmarks/configuration/output_format.md).
