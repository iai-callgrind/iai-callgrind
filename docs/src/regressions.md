<!-- markdownlint-disable MD041 MD033 -->

# Performance Regressions

With Iai-Callgrind you can define limits for each event kinds over which a
performance regression can be assumed. Per default, Iai-Callgrind does not
perform default regression checks, and you have to opt-in with a
`RegressionConfig` at benchmark level with a `LibraryBenchmarkConfig` or
`BinaryBenchmarkConfig` or at a global level with [Command-line arguments or
Environment variables](./cli_and_env/basics.md).

## Define a performance regression

A performance regression check consists of an `EventKind` and a percentage. If
the percentage is negative, then a regression is assumed to be below this limit.

The default `EventKind` is `EventKind::Ir` with a value of `+10%`.

For example, in a [Library
Benchmark](./benchmarks/library_benchmarks/configuration.md), define a limit of
`+5%` for the total instructions executed (the `Ir` event kind) in all
benchmarks of this file :

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    RegressionConfig, EventKind
};
use std::hint::black_box;

#[library_benchmark]
fn bench_library() -> Vec<i32> {
    black_box(my_lib::bubble_sort(vec![3, 2, 1]))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = my_group
);
# }
```

Now, if the comparison of the `Ir` events of the current `bench_library`
benchmark run with the previous run results in an increase of over 5%, the
benchmark fails. Please, also have a look at the [`api
docs`](https://docs.rs/iai-callgrind/0.14.1/iai_callgrind/struct.RegressionConfig.html)
for further configuration options.

Running the benchmark from above the first time results in the following output:

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::bench_library</span>
  Instructions:     <b>            215</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            288</b>|N/A             (<span style="color:#555">*********</span>)
  L2 Hits:          <b>              0</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              7</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            295</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            533</b>|N/A             (<span style="color:#555">*********</span>)</code></pre>

Let's assume there's a change in `my_lib::bubble_sort` which has increased the
instruction counts, then running the benchmark again results in an output
something similar to this:

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::bench_library</span>
  Instructions:     <b>            281</b>|215             (<b><span style="color:#F55">+30.6977%</span></b>) [<b><span style="color:#F55">+1.30698x</span></b>]
  L1 Hits:          <b>            374</b>|288             (<b><span style="color:#F55">+29.8611%</span></b>) [<b><span style="color:#F55">+1.29861x</span></b>]
  L2 Hits:          <b>              0</b>|0               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              8</b>|7               (<b><span style="color:#F55">+14.2857%</span></b>) [<b><span style="color:#F55">+1.14286x</span></b>]
  Total read+write: <b>            382</b>|295             (<b><span style="color:#F55">+29.4915%</span></b>) [<b><span style="color:#F55">+1.29492x</span></b>]
  Estimated Cycles: <b>            654</b>|533             (<b><span style="color:#F55">+22.7017%</span></b>) [<b><span style="color:#F55">+1.22702x</span></b>]
Performance has <b><span style="color:#F55">regressed</span></b>: <b>Instructions</b> (281 > 215) regressed by <b><span style="color:#F55">+30.6977%</span></b> (><span style="color:#555">+5.00000</span>)
iai_callgrind_runner: <b><span style="color:#A00">Error</span></b>: Performance has regressed.
error: bench failed, to rerun pass `-p the-crate --bench my_benchmark`

Caused by:
  process didn't exit successfully: `/path/to/your/project/target/release/deps/my_benchmark-a9b36fec444944bd --bench` (exit status: 1)
error: Recipe `bench-test` failed on line 175 with exit code 1</code></pre>

## Which event to choose to measure performance regressions?

If in doubt, the definite answer is `Ir` (instructions executed). If `Ir` event
counts decrease noticeable the function (binary) runs faster. The inverse
statement is also true: If the `Ir` counts increase noticeable, there's a
slowdown of the function (binary).

These statements are not so easy to transfer to `Estimated Cycles` and the other
event counts. But, depending on the scenario and the function (binary) under
test, it can be reasonable to define more regression checks.

## Who actually uses instructions to measure performance?

The ones known to the author of this humble guide are

* [SQLite](https://sqlite.org/cpu.html#performance_measurement): They use mainly
  cpu instructions to measure performance improvements (and regressions).
* Also in benchmarks of the [rustc](https://github.com/rust-lang/rustc-perf)
  compiler, instruction counts play a great role. But, they also use cache
  metrics and cycles.

If you know of others, please feel free to
[add](https://github.com/iai-callgrind/iai-callgrind/master/docs/src/regressions.md)
them to this list.
