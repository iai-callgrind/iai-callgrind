# Introduction

This is the guide for Iai-Callgrind, a benchmarking framework/harness which uses
[Valgrind's Callgrind](https://valgrind.org/docs/manual/cl-manual.html) and
other Valgrind tools like DHAT, Massif, ... to provide extremely accurate and
consistent measurements of Rust code, making it perfectly suited to run in
environments like a CI.

Iai_Callgrind is fully documented in this guide and in the api documentation at
[docs.rs](https://docs.rs/iai-callgrind/latest/iai_callgrind/).

Iai-Callgrind is

- __Precise__: High-precision measurements of `Instruction` counts and many
  other metrics allow you to reliably detect very small optimizations and
  performance and heap memory usage regressions of your code.
- __Consistent__: Iai-Callgrind can take accurate measurements even in
  virtualized CI environments and make them comparable between different systems
  completely negating the noise of the environment.
- __Fast__: Each benchmark is only run once, which is usually much faster than
  benchmarks which measure execution and wall time, since wall time benchmarks
  have to be run many times to increase their accuracy, detect outliers,
  filter out noise, etc.
- __Visualizable__: Iai-Callgrind generates a Callgrind (DHAT, ...) profile of
  the benchmarked code and can be configured to create flamegraph-like charts
  from Callgrind metrics. In general, all Valgrind-compatible tools like
  [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options),
  [kcachegrind](https://kcachegrind.github.io/html/Home.html) or `dh_view.html`
  and others to analyze the results in detail are fully supported.
- __Easy__: The api to setup benchmarks is easy to use but also powerful making
  complicated setups realizable. Concentrate more on profiling and your code
  than on the framework.

## Design philosophy

Iai-Callgrind benchmarks are designed to be runnable with `cargo bench`. The
benchmark files are expanded to a benchmarking harness which replaces the native
benchmark harness of `rust`. Iai-Callgrind abstracts the complicated parts and
repetitive tasks away and provides an easy to use and intuitive api.
Nevertheless, all configuration options for the `valgrind` invocation should be
easily accessible for the more sophisticated and advanced uses. The most
important rule for the overall design is: Concentrate more on profiling and your
code than on the framework.

## When not to use Iai-Callgrind

Although Iai-Callgrind is useful in many projects, there are cases where
Iai-Callgrind is not a good fit.

- If you need wall times, Iai-Callgrind cannot help you much. The estimation of
  cpu cycles is not a replacement for wall times and is primarily designed to be
  a relative metric to be used for comparison.
- Iai-Callgrind cannot be run on Windows and platforms not supported by
  Valgrind.
