<!-- markdownlint-disable MD041 MD033 -->

# Detecting Performance Regressions

With Iai-Callgrind you can define limits for each callgrind/cachegrind event
kind or dhat metric over which a performance regression can be assumed. Per
default, Iai-Callgrind does not perform regression checks, and you have to
opt-in with `Callgrind::soft_limits`, `Callgrind::hard_limits`,
`Cachegrind::soft_limits`, ... at benchmark level in
`LibraryBenchmarkConfig::tool` or `BinaryBenchmarkConfig::tool` or at a more
global level with [Command-line arguments or Environment
variables](./cli_and_env/basics.md), see
[below](#defining-limits-on-the-command-line).

For a soft limit, a performance regression check consists of an [`EventKind`],
[`CachegrindMetric`] or [`DhatMetric`] and a percentage. If the percentage is
negative, then a regression is assumed to be below this limit. Hard limits
restrict the `EventKind`, ... by an absolute number.

Note that [comparing baselines](./cli_and_env/baselines.md) also detects
performance regressions. This can be useful, for example, when setting up
Iai-Callgrind in the [CI](./installation/iai_callgrind.md#in-the-github-ci) to
cause a PR to fail when comparing to the main branch.

Regressions are considered errors and will cause the benchmark to fail if they
occur, and Iai-Callgrind will exit with error code `3`.

## Defining limits on the command-line

Limits can be defined on the command-line for the following tools with
`--callgrind-limits` (`IAI_CALLGRIND_CALLGRIND_LIMITS`), `--cachegrind-limits`
(`IAI_CALLGRIND_CACHEGRIND_LIMITS`)  and `--dhat-limits`
(`IAI_CALLGRIND_DHAT_LIMITS`). Command-line limits overwrite the limits
specified in the benchmark file (see below).

In order to disambiguate between soft and hard limits, soft limits have to be
suffixed with a `%`. Hard limits are bare numbers. For example to limit the
total instructions executed `ir` (printed as `Instructions` in the callgrind
terminal output) to `5%`:

```shell
cargo bench --bench iai_callgrind_benchmark -- --callgrind-limits='ir=5%'
```

These command-line arguments and environment variables can be used to define
soft limits and hard limits in one go with the `|`-operator (e.g.
`--callgrind-limits='ir=5%|10000'`) or multiple limits at once separated by a
`,` (e.g. `--callgrind-limits='ir=5%|10000,totalrw=2%'`).

For a list of all allowed callgrind metrics (like `ir`) see the docs of
[`EventKind`], for cachegrind metrics [`CachegrindMetric`] and for dhat metrics
[`DhatMetric`]. It is sometimes more convenient to define limits for whole
groups with the `@`-operator: `--callgrind-metrics='@all=5%'`. All allowed
groups and their members for callgrind metrics can be found in
[`CallgrindMetrics`], for cachegrind metrics in [`CachegrindMetrics`] and dhat
metrics in [`DhatMetrics`].

Multiple specifications of the same `EventKind`, ... overwrite the previous one
until the last one wins. This is useful for example to specify a limit for all
event kinds and then overwrite the limit for a specific event kind:
`--callgrind-limits='@all=10%,ir=5%'`

### The format, short names and groups in full detail

For `--callgrind-limits`:

```text
arg        ::= pair ("," pair)*
pair       ::= key "=" value ("|" value)*
key        ::= group | event         ; matched case-insensitive
group      ::= "@" ( "default"
                   | "all"
                   | ("cachemisses" | "misses" | "ms")
                   | ("cachemissrates" | "missrates" | "mr")
                   | ("cachehits" | "hits" | "hs")
                   | ("cachehitrates" | "hitrates" | "hr")
                   | ("cachesim" | "cs")
                   | ("cacheuse" | "cu")
                   | ("systemcalls" | "syscalls" | "sc")
                   | ("branchsim" | "bs")
                   | ("writebackbehaviour" | "writeback" | "wb")
                   )
event      ::= EventKind
value      ::= soft_limit | hard_limit
soft_limit ::= (integer | float) "%" ; can be negative
hard_limit ::= (integer | float)     ; float is only allowed for EventKinds which are
                                   ; float like `L1HitRate` but not `L1Hits`
```

with:

* Groups with a long name have their allowed abbreviations placed in the same
  parentheses.
* [`EventKind`] is the exact name of the enum variant (case insensitive)
* `integer` is a `u64` and `float` is a `f64`

For `--cachegrind-limits` replace the `group` and `event` from above with:

```text
group ::= "@" ( "default"
              | "all"
              | ("cachemisses" | "misses" | "ms")
              | ("cachemissrates" | "missrates" | "mr")
              | ("cachehits" | "hits" | "hs")
              | ("cachehitrates" | "hitrates" | "hr")
              | ("cachesim" | "cs")
              | ("branchsim" | "bs")
              )

event ::= CachegrindMetric
```

For `--dhat-limits` replace the `group` and `event` from above with:

```text
group ::= "@" ( "default" | "all" )
event ::= ( "totalunits" | "tun" )
          | ( "totalevents" | "tev" )
          | ( "totalbytes" | "tb" )
          | ( "totalblocks" | "tbk" )
          | ( "attgmaxbytes" | "gb" )
          | ( "attgmaxblocks" | "gbk" )
          | ( "attendbytes" | "eb" )
          | ( "attendblocks" | "ebk" )
          | ( "readsbytes" | "rb" )
          | ( "writesbytes" | "wb" )
          | ( "totallifetimes" | "tl" )
          | ( "maximumbytes" | "mb" )
          | ( "maximumblocks" | "mbk" )
```

## Define a performance regression check in a benchmark

For example, in a [Library
Benchmark](./benchmarks/library_benchmarks/configuration.md), define a soft
limit of `+5%` for the `Ir` event kind for all benchmarks of this file:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    Callgrind, EventKind
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
        .tool(Callgrind::default()
            .soft_limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = my_group
);
# }
```

Now, if the comparison of the `Ir` events of the current `bench_library`
benchmark run with the previous run results in an increase of over 5%, the
benchmark fails. Running the benchmark from above the first time results in the
following output:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_regression::my_group::bench_library</span> <span style="color:#0AA">worst_case</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [3, 2, 1]</span></b>
<span style="color:#555">  </span>Instructions:                         <b>152</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                              <b>201</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>LL Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
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
<span style="color:#555">  </span>LL Hits:                                <b>0</b>|0                    (<span style="color:#555">No change</span>)
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

For callgrind/cachegrind and if in doubt, the answer is `Ir` (instructions
executed). If `Ir` event counts decrease *noticeable* the function (binary) runs
faster. The inverse statement is also true: If the `Ir` counts increase
*noticeable*, there's a slowdown of the function (binary).

These statements are not so easy to transfer to `Estimated Cycles`, cache
metrics and most of the other event counts. But, depending on the scenario and
the function (binary) under test, it can be reasonable to define more regression
checks.

## Who actually uses instructions to measure performance?

The ones known to the author of this humble guide are

* [SQLite](https://sqlite.org/cpu.html#performance_measurement): They use mainly
  cpu instructions to measure performance improvements (and regressions).
* Also in benchmarks of the [rustc](https://github.com/rust-lang/rustc-perf)
  compiler and
  [compiler-builtins](https://github.com/rust-lang/compiler-builtins),
  instruction counts play a great role. But, they also use cache metrics and
  cycles.
* [SpacetimeDB](https://github.com/clockworklabs/SpacetimeDB)

If you know of others, please feel free to
[add](https://github.com/iai-callgrind/iai-callgrind/blob/5bec95ee37330954916ea29e7a7dc40ca62bc454/docs/src/regressions.md)
them to this list.

[`EventKind`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.EventKind.html
[`CallgrindMetrics`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.CallgrindMetrics.html
[`CachegrindMetric`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.CachegrindMetric.html
[`CachegrindMetrics`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.CachegrindMetrics.html
[`DhatMetric`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.DhatMetric.html
[`DhatMetrics`]: https://docs.rs/iai-callgrind/0.15.2/iai_callgrind/enum.DhatMetrics.html
