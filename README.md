<!-- spell-checker: ignore fixt binstall libtest eprintln usize Gjengset -->
<!-- markdownlint-disable MD041 MD033 -->

<h1 align="center">Iai-Callgrind</h1>

<div align="center">High-precision and consistent benchmarking framework/harness for Rust</div>

> **[!WARNING]**
> We've outgrown our original name. **Iai‑Callgrind** is transitioning to
> **Gungraun**. Starting with version 0.17.0, the crate will be published as
> **Gungraun**; earlier releases remain available under the old name. Thank you
> for your understanding — we look forward to continuing development under the new
> name!

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
        <img src="https://img.shields.io/badge/MSRV-1.74.1-brightgreen" alt="MSRV"/>
    </a>
</div>


Iai-Callgrind is a benchmarking framework/harness which uses [Valgrind's
Callgrind][Callgrind Manual] and other Valgrind tools like DHAT, Massif, ...
including Cachegrind to provide extremely accurate and consistent measurements
of Rust code, making it perfectly suited to run in environments like a CI.
Iai-Callgrind is integrated in [Bencher].

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

## Quickstart/Documentation

To get started read the [Guide] and see some introductory examples in [Quickstart
for library
benchmarks](https://iai-callgrind.github.io/iai-callgrind/latest/html/benchmarks/library_benchmarks/quickstart.html)
or [Quickstart for binary
benchmarks](https://iai-callgrind.github.io/iai-callgrind/latest/html/benchmarks/binary_benchmarks/quickstart.html)

## Design philosophy and goals

Iai-Callgrind benchmarks are designed to be runnable with `cargo bench`. The
benchmark files are expanded to a benchmarking harness which replaces the native
benchmark harness of `rust`. Iai-Callgrind is a profiling framework that can
quickly and reliably detect performance regressions and optimizations even in
noisy environments with a precision that is impossible to achieve with
wall-clock time based benchmarks. At the same time, we want to abstract the
complicated parts and repetitive tasks away and provide an easy to use and
intuitive api. Iai-Callgrind tries to stay out of your way and applies sensible
default settings so you can focus more on profiling and your code!

## How far are we?

Iai-Callgrind is in a mature development stage and is already [in
use](https://github.com/iai-callgrind/iai-callgrind/network/dependents).
Nevertheless, you may experience big changes between a minor version bump. With
the release of `0.14.0`, almost all `Callgrind` capabilities are implemented
including benchmarking of multi-threaded and multi-process applications. Using
`Cachegrind` instead of or in addition to `Callgrind` is possible since
`0.15.0`. Profiling of heap usage with `DHAT` is fully integrated since
`0.16.0`. Profiling with `Massif` is possible, but doesn't show useful metrics
in the terminal output and can be further improved. Creating callgrind
flamegraphs for multi-process/multi-threaded benchmarks is considered to be in
an experimental state. Please read our [Vision](./VISION.md) to learn more about
the ideas and the direction the future path might take.

## When not to use Iai-Callgrind

Although Iai-Callgrind is useful in many projects, there are cases where
Iai-Callgrind is not a good fit.

- If you need wall-clock times, Iai-Callgrind cannot help you much. The
  estimation of cpu cycles merely correlates to wall-clock times but is not a
  replacement for wall-clock times. The cycles estimation is primarily designed
  to be a relative metric to be used for comparison.
- Iai-Callgrind cannot be run on Windows and platforms not supported by
  Valgrind.

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
  Iai-Callgrind was initially forked to use Callgrind instead of Cachegrind as
  primary profiling tool. See [Comparison][Comparison.iai].
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

Iai-Callgrind is forked from <https://github.com/bheisler/iai> and the original
idea is from Brook Heisler (@bheisler).

Iai-Callgrind is powered by [Valgrind].

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

[Comparison.iai]: https://iai-callgrind.github.io/iai-callgrind/latest/html/comparison/iai.html

[kcachegrind]: https://kcachegrind.github.io/html/Home.html

[Valgrind]: https://valgrind.org/

[Valgrind Client Request Mechanism]: https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq

[Callgrind Manual]: https://valgrind.org/docs/manual/cl-manual.html

[Callgrind Annotate]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options
