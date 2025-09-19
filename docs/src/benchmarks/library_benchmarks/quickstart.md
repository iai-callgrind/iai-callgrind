<!-- markdownlint-disable MD041 MD033 -->

# Quickstart

Create a file `$WORKSPACE_ROOT/benches/library_benchmark.rs` and add

```toml
[[bench]]
name = "library_benchmark"
harness = false
```

to your `Cargo.toml`. `harness = false`, tells `cargo` to not use the default
rust benchmarking harness which is important because Gungraun has an own
benchmarking harness.

Then copy the following content into this file:

```rust
# extern crate iai_callgrind;
use iai_callgrind::{main, library_benchmark_group, library_benchmark};
use std::hint::black_box;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[library_benchmark]
#[bench::short(10)]
#[bench::long(30)]
fn bench_fibonacci(value: u64) -> u64 {
    black_box(fibonacci(value))
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench_fibonacci
);

# fn main() {
main!(library_benchmark_groups = bench_fibonacci_group);
# }
```

Now, that your first library benchmark is set up, you can run it with

```shell
cargo bench
```

and should see something like the below

<pre><code class="hljs"><span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
  Instructions:     <b>           1734</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>           2359</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              0</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              3</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>           2362</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>           2464</b>|N/A             (<span style="color:#555">*********</span>)
<span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">long</span><span style="color:#0AA">:</span><b><span style="color:#00A">30</span></b>
  Instructions:     <b>       26214734</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>       35638616</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              2</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>       35638622</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>       35638766</b>|N/A             (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 2 without regressions; 0 regressed; 2 benchmarks finished in 0.49333s</code></pre>

In addition, you'll find the callgrind output and the output of other valgrind
tools in `target/iai`, if you want to investigate further with a tool like
`callgrind_annotate` etc.

When running the same benchmark again, the output will report the differences
between the current and the previous run. Say you've made change to the
`fibonacci` function, then you may see something like this:

<pre><code class="hljs"><span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
  Instructions:     <b>           2805</b>|1734            (<b><span style="color:#F55">+61.7647%</span></b>) [<b><span style="color:#F55">+1.61765x</span></b>]
  L1 Hits:          <b>           3815</b>|2359            (<b><span style="color:#F55">+61.7211%</span></b>) [<b><span style="color:#F55">+1.61721x</span></b>]
  LL Hits:          <b>              0</b>|0               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              3</b>|3               (<span style="color:#555">No change</span>)
  Total read+write: <b>           3818</b>|2362            (<b><span style="color:#F55">+61.6427%</span></b>) [<b><span style="color:#F55">+1.61643x</span></b>]
  Estimated Cycles: <b>           3920</b>|2464            (<b><span style="color:#F55">+59.0909%</span></b>) [<b><span style="color:#F55">+1.59091x</span></b>]
<span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">long</span><span style="color:#0AA">:</span><b><span style="color:#00A">30</span></b>
  Instructions:     <b>       16201597</b>|26214734        (<b><span style="color:#42c142">-38.1966%</span></b>) [<b><span style="color:#42c142">-1.61803x</span></b>]
  L1 Hits:          <b>       22025876</b>|35638616        (<b><span style="color:#42c142">-38.1966%</span></b>) [<b><span style="color:#42c142">-1.61803x</span></b>]
  LL Hits:          <b>              2</b>|2               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              4</b>|4               (<span style="color:#555">No change</span>)
  Total read+write: <b>       22025882</b>|35638622        (<b><span style="color:#42c142">-38.1966%</span></b>) [<b><span style="color:#42c142">-1.61803x</span></b>]
  Estimated Cycles: <b>       22026026</b>|35638766        (<b><span style="color:#42c142">-38.1964%</span></b>) [<b><span style="color:#42c142">-1.61803x</span></b>]

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 2 without regressions; 0 regressed; 2 benchmarks finished in 0.49333s</code></pre>
