# Introduction

This is the guide for Iai-Callgrind, a benchmarking framework/harness which uses
[Valgrind's Callgrind](https://valgrind.org/docs/manual/cl-manual.html) to
provide extremely accurate and consistent measurements of Rust code, making it
perfectly suited to run in environments like a CI. Iai-Callgrind is flexible and
despite its name it's possible to run [Cachegrind](./cachegrind.md) or any other
[Valgrind tool](./tools.md) like DHAT in addition to or instead of Callgrind.

Iai-Callgrind is fully documented in this guide and in the api documentation at
[docs.rs](https://docs.rs/iai-callgrind/latest/iai_callgrind/).

Iai-Callgrind is also:

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
  [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options),
  [kcachegrind](https://kcachegrind.github.io/html/Home.html) or `dh_view.html`
  and others to analyze the results in detail are fully supported.
- __Easy__: The API for setting up benchmarks is easy to use and allows you to
  quickly create concise and clear benchmarks. Focus more on profiling and your
  code than on the framework.

## Design philosophy and goals

Iai-Callgrind benchmarks are designed to be runnable with `cargo bench`. The
benchmark files are expanded to a benchmarking harness which replaces the native
benchmark harness of `rust`. Iai-Callgrind is a profiling framework that can
quickly and reliably detect performance regressions and optimizations even in
noisy environments with a precision that is impossible to achieve with
wall-clock time based benchmarks. At the same time, we want to abstract the
complicated parts and repetitive tasks away and provide an easy to use and
intuitive api. Iai-Callgrind tries to stay out of your way so you can focus more
on profiling and your code!

## When not to use Iai-Callgrind

Although Iai-Callgrind is useful in many projects, there are cases where
Iai-Callgrind is not a good fit.

- If you need wall-clock times, Iai-Callgrind cannot help you much. The
  estimation of cpu cycles merely correlates to wall-clock times but is not a
  replacement for wall-clock times. The cycles estimation is primarily designed
  to be a relative metric to be used for comparison.
- Iai-Callgrind cannot be run on Windows and platforms not supported by
  Valgrind.

## Improving Iai-Callgrind

You want to improve the guide? You have an idea for a new feature, are missing a
functionality or have found a bug? We would love to here about it. You want to
contribute and hack on Iai-Callgrind?

Please don't hesitate to [open an
issue](https://github.com/iai-callgrind/iai-callgrind/issues).

You want to hack on this guide? The source code of this book lives in [the docs
subdirectory](https://github.com/iai-callgrind/iai-callgrind/tree/main/docs).
