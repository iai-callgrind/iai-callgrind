<!-- spell-checker: ignore fixt binstall libtest eprintln usize Gjengset -->
<!-- markdownlint-disable MD041 MD033 -->

<h1 align="center">Iai-Callgrind</h1>

<div align="center">High-precision and consistent benchmarking framework/harness for Rust</div>

<div align="center">
    <a href="https://iai-callgrind.github.io/iai-callgrind">Guide</a>
    |
    <a href="https://docs.rs/crate/iai-callgrind/">Released API Docs</a>
    |
    <a href="https://github.com/iai-callgrind/iai-callgrind/blob/main/CHANGELOG.md">Changelog</a>
</div>
<br>
<div align="center">
    <a href="https://github.com/iai-callgrind/iai-callgrind/actions/workflows/cicd.yml">
        <img src="https://github.com/iai-callgrind/iai-callgrind/actions/workflows/cicd.yml/badge.svg" alt="GitHub branch checks state"/>
    </a>
    <a href="https://crates.io/crates/iai-callgrind">
        <img src="https://img.shields.io/crates/v/iai-callgrind.svg" alt="Crates.io"/>
    </a>
    <a href="https://docs.rs/iai-callgrind/">
        <img src="https://docs.rs/iai-callgrind/badge.svg" alt="docs.rs"/>
    </a>
    <a href="https://github.com/rust-lang/rust">
        <img src="https://img.shields.io/badge/MSRV-1.75.0-brightgreen" alt="MSRV"/>
    </a>
</div>

Iai-Callgrind is a benchmarking framework/harness which uses [Valgrind's
Callgrind][Callgrind Manual] and other Valgrind tools like DHAT, Massif, ... to
provide extremely accurate and consistent measurements of Rust code, making it
perfectly suited to run in environments like a CI. Iai-Callgrind is integrated
in [Bencher].

Iai-Callgrind is:

- __Precise__: High-precision measurements of `Instruction` counts and many
  other metrics allow you to reliably detect very small optimizations and
  regressions of your code.
- __Consistent__: Iai-Callgrind can take accurate measurements even in
  virtualized CI environments and make them comparable between different systems
  completely negating the noise of the environment.
- __Fast__: Each benchmark is only run once, which is usually much faster than
  benchmarks which measure execution and wall-clock time. Benchmarks measuring
  the wall-clock time have to be run many times to increase their accuracy,
  detect outliers, filter out noise, etc.
- __Visualizable__: Iai-Callgrind generates a Callgrind (DHAT, ...) profile of
  the benchmarked code and can be configured to create flamegraph-like charts
  from Callgrind metrics. In general, all Valgrind-compatible tools like
  [callgrind_annotate][Callgrind Annotate], [kcachegrind] or `dh_view.html` and
  others to analyze the results in detail are fully supported.
- __Easy__: The API for setting up benchmarks is easy to use and allows you to
  quickly create concise and clear benchmarks. Focus more on profiling and your
  code than on the framework.

See the [Guide] and api documentation at [docs.rs][Api Docs] for all the
details.

## Design philosophy and goals

Iai-Callgrind benchmarks are designed to be runnable with `cargo bench`. The
benchmark files are expanded to a benchmarking harness which replaces the native
benchmark harness of `rust`. Iai-Callgrind is a profiling framework that can
quickly and reliably detect performance regressions and optimizations even in
noisy environments with a precision that is impossible to achieve with
wall-clock time based benchmarks. At the same time, we want to abstract the
complicated parts and repetitive tasks away and provide an easy to use and
intuitive api. Concentrate more on profiling and your code than on the
framework!

## When not to use Iai-Callgrind

Although Iai-Callgrind is useful in many projects, there are cases where
Iai-Callgrind is not a good fit.

- If you need wall-clock times, Iai-Callgrind cannot help you much. The
  estimation of cpu cycles merely correlates to wall-clock times but is not a
  replacement for wall-clock times. The cycles estimation is primarily designed
  to be a relative metric to be used for comparison.
- Iai-Callgrind cannot be run on Windows and platforms not supported by
  Valgrind.

## Quickstart

You're missing the old README? To get started read the [Guide].

The guide maintains only the versions `0.12.3` upwards. For older versions
checkout the README of this repo using a specific tagged version for example
<https://github.com/iai-callgrind/iai-callgrind/tree/v0.12.2> or using the
github ui.

Here's just a small introductory example, assuming you have everything
[installed][Guide Prerequisites] and a benchmark with the following content in
`benches/library_benchmark.rs` ready:

```rust
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

library_benchmark_group!(name = bench_fibonacci_group; benchmarks = bench_fibonacci);
main!(library_benchmark_groups = bench_fibonacci_group);
```

Now run

```shell
cargo bench
```

<pre><code class="hljs"><span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">short</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
  Instructions:     <b>           1734</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>           2359</b>|N/A             (<span style="color:#555">*********</span>)
  L2 Hits:          <b>              0</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              3</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>           2362</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>           2464</b>|N/A             (<span style="color:#555">*********</span>)
<span style="color:#0A0">library_benchmark::bench_fibonacci_group::bench_fibonacci</span> <span style="color:#0AA">long</span><span style="color:#0AA">:</span><b><span style="color:#00A">30</span></b>
  Instructions:     <b>       26214734</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>       35638616</b>|N/A             (<span style="color:#555">*********</span>)
  L2 Hits:          <b>              2</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>       35638622</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>       35638766</b>|N/A             (<span style="color:#555">*********</span>)</code></pre>

### Contributing

Thanks for helping to improve this project! A guideline about contributing to
Iai-Callgrind can be found in the [CONTRIBUTING.md](./CONTRIBUTING.md) file.

You have an idea for a new feature, are missing a functionality or have found a
bug?

Please don't hesitate to [open an
issue](https://github.com/iai-callgrind/iai-callgrind/issues).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as in
[License](#license), without any additional terms or conditions.

### Links

- Iai-Callgrind is [mentioned](https://youtu.be/qfknfCsICUM?t=1228) in a talk at
  [RustNation UK](https://www.rustnationuk.com/) about [Towards Impeccable
  Rust](https://www.youtube.com/watch?v=qfknfCsICUM) by Jon Gjengset
- Iai-Callgrind is supported by [Bencher]

### Related Projects

- [Iai](https://github.com/bheisler/iai): The repository from which
  Iai-Callgrind is forked. Iai uses Cachegrind instead of Callgrind under the
  hood.
- [Criterion-rs](https://github.com/bheisler/criterion.rs): A Statistics-driven
  benchmarking library for Rust. Wall-clock times based benchmarks.
- [hyperfine](https://github.com/sharkdp/hyperfine): A command-line benchmarking
  tool. Wall-clock time based benchmarks.
- [divan](https://github.com/nvzqz/divan): Statistically-comfy benchmarking
  library. Wall-clock times based benchmarks.
- [dhat-rs](https://github.com/nnethercote/dhat-rs): Provides heap profiling and
  ad hoc profiling capabilities to Rust programs, similar to those provided by
  DHAT.
- [cargo-valgrind](https://github.com/jfrimmel/cargo-valgrind): A cargo
  subcommand, that runs valgrind and collects its output in a helpful manner.
- [crabgrind](https://github.com/2dav/crabgrind): Valgrind Client Request
  interface for Rust programs. A small library that enables Rust programs to tap
  into Valgrind's tools and virtualized environment.

### Credits

Iai-Callgrind is forked from <https://github.com/bheisler/iai> and was
originally written by Brook Heisler (@bheisler).

Iai-Callgrind wouldn't be possible without [Valgrind].

### License

Iai-Callgrind is like Iai dual licensed under the Apache 2.0 license and the MIT
license at your option.

According to [Valgrind's documentation][Valgrind Client Request Mechanism]:

> The Valgrind headers, unlike most of the rest of
> the code, are under a BSD-style license, so you may include them without worrying
> about license incompatibility.

We have included the original license where we made use of the original header
files.

[Api Docs]: https://docs.rs/iai-callgrind/latest/iai_callgrind/

[Bencher]: https://bencher.dev/learn/benchmarking/rust/iai/

[Guide]: https://iai-callgrind.github.io/iai-callgrind/

[Guide Prerequisites]: https://iai-callgrind.github.io/iai-callgrind/latest/html/installation/prerequisites.html

[kcachegrind]: https://kcachegrind.github.io/html/Home.html

[Valgrind]: https://valgrind.org/

[Valgrind Client Request Mechanism]: https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq

[Callgrind Manual]: https://valgrind.org/docs/manual/cl-manual.html

[Callgrind Annotate]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options
