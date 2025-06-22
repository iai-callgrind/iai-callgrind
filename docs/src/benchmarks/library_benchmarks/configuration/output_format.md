<!-- markdownlint-disable MD041 MD033 -->

# Output Format

The Iai-Callgrind output can be customized with [command-line
arguments](../../../cli_and_env/output.md). But, the fine-grained terminal
output format is adjusted in the benchmark itself. For example [truncating
the description][`OutputFormat.truncate_description`], [showing a
grid][`OutputFormat.show_grid`], .... Please read the [docs][`OutputFormat`] for
further details.

However, I want to point out the possibility to show the cache misses in the
Iai-Callgrind output in the following section.

## Showing cache misses

A default Iai-Callgrind benchmark run displays the following metrics:

<pre><code class="hljs"><span style="color:#0A0">test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
<span style="color:#555">  </span>Instructions:                        <b>1734</b>|1734                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>2359</b>|2359                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Total read+write:                    <b>2362</b>|2362                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Estimated Cycles:                    <b>2464</b>|2464                 (<span style="color:#555">No change</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

The cache and ram hits, `Total read+write` and `Estimated Cycles` are actually
not part of the original collected callgrind metrics but calculated from them.
If you want to see the cache misses nonetheless, you can achieve this by
specifying the output format for example at top-level for all benchmarks in the
same file in the `main!` macro:

```rust
# extern crate iai_callgrind;
# use iai_callgrind::{library_benchmark, library_benchmark_group};
use iai_callgrind::{main, LibraryBenchmarkConfig, OutputFormat, CallgrindMetrics, Callgrind};

# #[library_benchmark] fn bench() {}
# library_benchmark_group!(name = my_group; benchmarks = bench);
# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::default()
            .format([CallgrindMetrics::All])
        );
    library_benchmark_groups = my_group
);
# }
```

The Iai-Callgrind output will then show all cache metrics:

<pre><code class="hljs"><span style="color:#0A0">test_lib_bench_readme_example_fibonacci::my_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
<span style="color:#555">  </span>Instructions:                        <b>1734</b>|1734                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Dr:                                   <b>270</b>|270                  (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Dw:                                   <b>358</b>|358                  (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>I1mr:                                   <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>D1mr:                                   <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>D1mw:                                   <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>ILmr:                                   <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>DLmr:                                   <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>DLmw:                                   <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>2359</b>|2359                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>3</b>|3                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Total read+write:                    <b>2362</b>|2362                 (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Estimated Cycles:                    <b>2464</b>|2464                 (<span style="color:#555">No change</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49301s</code></pre>

The callgrind output format can be fully customized showing only the metrics
you're interested in and in any order. The docs of `Callgrind::format` and
`CallgrindMetrics` show all the possibilities for `Callgrind`. The output format
of the other valgrind tools can be customized in the same way. Just have a look
at the docs for the respective format (`Dhat::format`, `DhatMetric`,
`Cachegrind::format`, `CachegrindMetric`, ...)

[`OutputFormat`]: https://docs.rs/iai-callgrind/0.15.0/iai_callgrind/struct.OutputFormat.html
[`OutputFormat.show_grid`]: https://docs.rs/iai-callgrind/0.15.0/iai_callgrind/struct.OutputFormat.html#method.show_grid
[`OutputFormat.truncate_description`]: https://docs.rs/iai-callgrind/0.15.0/iai_callgrind/struct.OutputFormat.html#method.truncate_description
