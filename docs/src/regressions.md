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
#[bench::worst_case(vec![3, 2, 1])]
fn bench_library(data: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(data))
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
docs`](https://docs.rs/iai-callgrind/0.14.2/iai_callgrind/struct.RegressionConfig.html)
for further configuration options.

Running the benchmark from above the first time results in the following output:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_regression::my_group::bench_library</span> <span style="color:#0AA">worst_case</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [3, 2, 1]</span></b>
<span style="color:#555">  </span>Instructions:                         <b>152</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                              <b>201</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>5</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                     <b>206</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                     <b>376</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.14477s</code></pre>

Let's assume there's a change in `my_lib::bubble_sort` with a negative impact on
the performance, then running the benchmark again results in an output something
similar to this:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_regression::my_group::bench_library</span> <span style="color:#0AA">worst_case</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [3, 2, 1]</span></b>
<span style="color:#555">  </span>Instructions:                         <b>264</b>|152                  (<b><span style="color:#F55">+73.6842%</span></b>) [<b><span style="color:#F55">+1.73684x</span></b>]
<span style="color:#555">  </span>L1 Hits:                              <b>341</b>|201                  (<b><span style="color:#F55">+69.6517%</span></b>) [<b><span style="color:#F55">+1.69652x</span></b>]
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>6</b>|5                    (<b><span style="color:#F55">+20.0000%</span></b>) [<b><span style="color:#F55">+1.20000x</span></b>]
<span style="color:#555">  </span>Total read+write:                     <b>347</b>|206                  (<b><span style="color:#F55">+68.4466%</span></b>) [<b><span style="color:#F55">+1.68447x</span></b>]
<span style="color:#555">  </span>Estimated Cycles:                     <b>551</b>|376                  (<b><span style="color:#F55">+46.5426%</span></b>) [<b><span style="color:#F55">+1.46543x</span></b>]
Performance has <b><span style="color:#F55">regressed</span></b>: <b>Instructions</b> (152 -> <b>264</b>) regressed by <b><span style="color:#F55">+73.6842%</span></b> (><span style="color:#555">+5.00000%</span>)

Regressions:

  <span style="color:#0A0">lib_bench_regression::my_group::bench_library</span>:
    <b>Instructions</b> (152 -> <b>264</b>): <b><span style="color:#F55">+73.6842</span></b><b><span style="color:#F55">%</span></b> exceeds limit of <span style="color:#555">+5.00000</span><span style="color:#555">%</span>

Iai-Callgrind result: <b><span style="color:#F55">Regressed</span></b>. 0 without regressions; 1 regressed; 1 benchmarks finished in 0.14849s
error: bench failed, to rerun pass `-p benchmark-tests --bench lib_bench_regression`

Caused by:
  process didn't exit successfully: `/home/lenny/workspace/programming/iai-callgrind/target/release/deps/lib_bench_regression-98382b533bca8f56 --bench` (exit status: 3)</code></pre>

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
