<h1 align="center">Iai-Callgrind</h1>

<div align="center">High-precision and consistent benchmarking framework/harness for Rust</div>

<div align="center">
    <a href="https://docs.rs/crate/iai-callgrind/">Released API Docs</a>
    |
    <a href="https://github.com/Joining7943/iai-callgrind/blob/main/CHANGELOG.md">Changelog</a>
</div>
<br>
<div align="center">
    <a href="https://github.com/Joining7943/iai-callgrind/actions/workflows/cicd.yml">
        <img src="https://github.com/Joining7943/iai-callgrind/actions/workflows/cicd.yml/badge.svg" alt="GitHub branch checks state"/>
    </a>
    <a href="https://crates.io/crates/iai-callgrind">
        <img src="https://img.shields.io/crates/v/iai-callgrind.svg" alt="Crates.io"/>
    </a>
    <a href="https://docs.rs/iai-callgrind/">
        <img src="https://docs.rs/iai-callgrind/badge.svg" alt="docs.rs"/>
    </a>
    <a href="https://github.com/rust-lang/rust">
        <img src="https://img.shields.io/badge/MSRV-1.56.0-brightgreen" alt="MSRV"/>
    </a>
</div>

Iai-Callgrind is a benchmarking framework and harness that uses Callgrind to provide extremely
accurate and consistent measurements of Rust code, making it perfectly suited to run in environments
like a CI.

This crate started as a fork of the great [Iai](https://github.com/bheisler/iai) crate rewritten to
use Valgrind's [Callgrind](https://valgrind.org/docs/manual/cl-manual.html) instead of
[Cachegrind](https://valgrind.org/docs/manual/cg-manual.html) but also adds a lot of other
improvements and features.

## Table of Contents

- [Table of Contents](#table-of-contents)
    - [Features](#features)
    - [Installation](#installation)
    - [Quickstart](#quickstart)
    - [Examples](#examples)
    - [Features and differences to Iai](#features-and-differences-to-iai)
    - [What hasn't changed](#what-hasnt-changed)
    - [See also](#see-also)
    - [Credits](#credits)
    - [License](#license)

### Features

- __Precision__: High-precision measurements allow you to reliably detect very small optimizations
to your code
- __Consistency__: Iai-Callgrind can take accurate measurements even in virtualized CI environments
- __Performance__: Since Iai-Callgrind only executes a benchmark once, it is typically faster to run
than benchmarks measuring the execution and wall time
- __Regression__: Iai-Callgrind reports the difference between benchmark runs to make it easy to
spot detailed performance regressions and improvements.
- __Profiling__: Iai-Callgrind generates a Callgrind profile of your code while benchmarking, so you
can use Callgrind-compatible tools like
[callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the results
in detail
- __Stable-compatible__: Benchmark your code without installing nightly Rust

### Installation

In order to use Iai-Callgrind, you must have [Valgrind](https://www.valgrind.org) installed. This
means that Iai-Callgrind cannot be used on platforms that are not supported by Valgrind.

To start with Iai-Callgrind, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
iai-callgrind = "0.2.0"
```

To be able to run the benchmarks you'll also need the `iai-callgrind-runner` binary installed
somewhere in your `$PATH`, for example with

```shell
cargo install --version 0.2.0 iai-callgrind-runner
```

When updating the `iai-callgrind` library, you'll also need to update `iai-callgrind-runner` and
vice-versa or else the benchmark runner will exit with an error.

### Quickstart

Add

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

to your `Cargo.toml` file and then create a file with the same `name` in `benches/my_benchmark.rs`
with the following content:

```rust
use iai_callgrind::{black_box, main};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

// Don't forget the `#[inline(never)]`
#[inline(never)]
fn iai_benchmark_short() -> u64 {
    fibonacci(black_box(10))
}

#[inline(never)]
fn iai_benchmark_long() -> u64 {
    fibonacci(black_box(30))
}

main!(iai_benchmark_short, iai_benchmark_long);
```

Note that it is important to annotate the benchmark functions with `#[inline(never)]` or else the
rust compiler will most likely try to optimize this function and inline it. `Callgrind` is function
(name) based and the collection of counter events starts when entering this function and ends when
leaving it. Not inlining this function serves the additional purpose to reduce influences of the
surrounding code on the benchmark function.

Now you can run this benchmark with `cargo bench --bench my_benchmark` in your project root and you
should see something like this:

```text
my_benchmark::bench_fibonacci_short
  Instructions:                1727
  L1 Data Hits:                 621
  L2 Hits:                        0
  RAM Hits:                       1
  Total read+write:            2349
  Estimated Cycles:            2383
my_benchmark::bench_fibonacci_long
  Instructions:            26214727
  L1 Data Hits:             9423880
  L2 Hits:                        0
  RAM Hits:                       2
  Total read+write:        35638609
  Estimated Cycles:        35638677
```

In addition, you'll find the callgrind output in `target/iai/my_benchmark`, if you want to
investigate further with a tool like `callgrind_annotate`. Now, if running the same benchmark again,
the output will report the differences between the current and the previous run. Say you've made
change to the `fibonacci` function, then you might see something like this:

```text
my_benchmark::bench_fibonacci_short
  Instructions:                2798 (+62.01506%)
  L1 Data Hits:                1006 (+61.99678%)
  L2 Hits:                        0 (No Change)
  RAM Hits:                       1 (No Change)
  Total read+write:            3805 (+61.98382%)
  Estimated Cycles:            3839 (+61.09945%)
my_benchmark::bench_fibonacci_long
  Instructions:            16201590 (-38.19661%)
  L1 Data Hits:             5824277 (-38.19661%)
  L2 Hits:                        0 (No Change)
  RAM Hits:                       2 (No Change)
  Total read+write:        22025869 (-38.19661%)
  Estimated Cycles:        22025937 (-38.19654%)
```

### Examples

For examples see also the [benches](iai-callgrind-runner/benches) folder.

#### Skipping setup code

Usually, all setup code in the benchmark function itself is attributed to the event counts. It's
possible to pass additional arguments to Callgrind and something like below will eliminate the setup
code from the final metrics:

```rust
use iai_callgrind::{black_box, main};
use my_library;

#[export_name = "some_special_id::expensive_setup"]
#[inline(never)]
fn expensive_setup() -> Vec<u64> {
    // some expensive setup code to produce a Vec<u64>
}

#[inline(never)]
fn test() {
    my_library::call_to_function(black_box(expensive_setup()));
}

main!(
    callgrind_args = "toggle-collect=some_special_id::expensive_setup";
    functions = test
);
```

and then run the benchmark for example with

```shell
cargo bench --bench my_bench
```

See also [Skip setup code example](iai-callgrind-runner/benches/test_with_skip_setup.rs) for an
in-depth explanation.

### Features and differences to Iai

This crate is built on the same idea like the original Iai, but over the time applied a lot of
improvements. The biggest difference is, that it uses Callgrind under the hood instead of
Cachegrind.

#### More stable metrics

Iai-Callgrind has even more precise and stable metrics across different systems. It achieves this by

- start counting the events when entering the benchmark function and stop counting when leaving the
benchmark function. The call to the benchmark function itself is also subtracted (See `bench_empty`
below). This behavior virtually encapsulates the benchmark function and (almost) completely
separates the benchmark from the surrounding code.
- separating the iai library with the main macro from the actual runner. This is the reason for the
extra installation step of `iai-callgrind-runner` but before this separation even small changes in
the iai library had effects on the benchmarks under test.

Below a run of one of the benchmarks of this library on my local computer

```shell
$ cd iai-callgrind
$ cargo bench --bench test_regular_bench
test_regular_bench::bench_empty
  Instructions:                   0
  L1 Data Hits:                   0
  L2 Hits:                        0
  RAM Hits:                       0
  Total read+write:               0
  Estimated Cycles:               0
test_regular_bench::bench_fibonacci
  Instructions:                1727
  L1 Data Hits:                 621
  L2 Hits:                        0
  RAM Hits:                       1
  Total read+write:            2349
  Estimated Cycles:            2383
test_regular_bench::bench_fibonacci_long
  Instructions:            26214727
  L1 Data Hits:             9423880
  L2 Hits:                        0
  RAM Hits:                       2
  Total read+write:        35638609
  Estimated Cycles:        35638677
```

For comparison here the output of the same benchmark but in the github CI:

```text
test_regular_bench::bench_empty
  Instructions:                   0
  L1 Data Hits:                   0
  L2 Hits:                        0
  RAM Hits:                       0
  Total read+write:               0
  Estimated Cycles:               0
test_regular_bench::bench_fibonacci
  Instructions:                1727
  L1 Data Hits:                 621
  L2 Hits:                        0
  RAM Hits:                       1
  Total read+write:            2349
  Estimated Cycles:            2383
test_regular_bench::bench_fibonacci_long
  Instructions:            26214727
  L1 Data Hits:             9423880
  L2 Hits:                        0
  RAM Hits:                       2
  Total read+write:        35638609
  Estimated Cycles:        35638677
```

There's no difference (in this example) what makes benchmark runs and performance improvements of
the benchmarked code even more comparable across systems. However, the above benchmarks are pretty
clean and you'll most likely see some very small differences in your own benchmarks.

#### Cleaner output of Valgrind's annotation tools

The now obsolete calibration run needed with Iai has just fixed the summary output of Iai itself,
but the output of `cg_annotate` was still cluttered by the setup functions and metrics. The
`callgrind_annotate` output produced by Iai-Callgrind is far cleaner and centered on the actual
function under test.

#### Rework the metrics output

The statistics of the benchmarks are mostly not compatible with the original Iai anymore although
still related. They now also include some additional information:

```text
test_regular_bench::bench_fibonacci_long
  Instructions:            26214732
  L1 Data Hits:             9423880
  L2 Hits:                        0
  RAM Hits:                       2
  Total read+write:        35638609
  Estimated Cycles:        35638677
```

There is an additional line `Total read+write` which summarizes all event counters above it and the
`L1 Accesses` line changed to `L1 Data Hits`. So, the (L1) `Instructions` (reads) and `L1 Data Hits`
are now separately listed.

In detail:

`Total read+write = Instructions + L1 Data Hits + L2 Hits + RAM Hits`.

The formula for the `Estimated Cycles` hasn't changed and uses Itamar Turner-Trauring's formula from
<https://pythonspeed.com/articles/consistent-benchmarking-in-ci/>:

`Estimated Cycles = (Instructions + L1 Data Hits) + 5 × (L2 Hits) + 35 × (RAM Hits)`

For further details about how the caches are simulated and more, see the documentation of
[Callgrind](https://valgrind.org/docs/manual/cg-manual.html)

#### Colored output and logging

The metrics output is colored per default but follows the value for the `CARGO_TERM_COLOR`
environment variable. Disabling colors can be achieved with setting this environment variable to
`CARGO_TERM_COLOR=never`.

This library uses [env_logger](https://crates.io/crates/env_logger) and the default logging level
`WARN`. Currently, `env_logger` is only used to print some warnings and debug output, but to set the
logging level to something different set the environment variable `RUST_LOG` for example to
`RUST_LOG=DEBUG`. The logging output is colored per default but follows the setting of
`CARGO_TERM_COLOR`. See also the [documentation](https://docs.rs/env_logger/latest) of `env_logger`.

#### Passing arguments to Callgrind

It's now possible to pass additional arguments to callgrind separated by `--`
(`cargo bench -- CALLGRIND_ARGS`) or overwrite the defaults, which are:

- `--I1=32768,8,64`
- `--D1=32768,8,64`
- `--LL=8388608,16,64`
- `--cache-sim=yes` (can't be changed)
- `--toggle-collect=*BENCHMARK_FILE::BENCHMARK_FUNCTION`
- `--collect-atstart=no`
- `--compress-pos=no`
- `--compress-strings=no`

Note that `toggle-collect` won't be overwritten by any additional `toggle-collect` argument but
instead will be passed to Callgrind in addition to the default value. See the [Skipping setup
code](#skipping-setup-code) section for an example of how to make use of this.

It's also possible to pass arguments to callgrind on a benchmark file level with the alternative
form of the main macro

```rust
main!(
    callgrind_args = "--arg-with-flags=yes", "arg-without-flags=is_ok_too"
    functions = func1, func2
)
```

See also [Callgrind Command-line Options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options).

#### Incomplete list of other minor improvements

- The output files of Callgrind are now located under a subdirectory under `target/iai` to avoid
  overwriting them in case of multiple benchmark files.

### What hasn't changed

Iai-Callgrind does not completely remove the influences of setup changes (like an additional
benchmark function in the same file). However, these effects shouldn't be so large anymore.

### See also

- The user guide of the original Iai: <https://bheisler.github.io/criterion.rs/book/iai/iai.html>
- A comparison of criterion-rs with Iai: <https://github.com/bheisler/iai#comparison-with-criterion-rs>

### Credits

Iai-Callgrind is forked from <https://github.com/bheisler/iai> and was originally written by Brook
Heisler (@bheisler).

### License

Iai-Callgrind is like Iai dual licensed under the Apache 2.0 license and the MIT license.
