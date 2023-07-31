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
        <img src="https://img.shields.io/badge/MSRV-1.60.0-brightgreen" alt="MSRV"/>
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
    - [Update notes](#update-notes)
    - [Installation](#installation)
    - [Benchmarking](#benchmarking)
        - [Library Benchmarks](#library-benchmarks)
        - [Binary Benchmarks](#binary-benchmarks)
    - [Features and differences to Iai](#features-and-differences-to-iai)
    - [What hasn't changed](#what-hasnt-changed)
    - [See also](#see-also)
    - [Credits](#credits)
    - [License](#license)

### Features

- __Precision__: High-precision measurements allow you to reliably detect very small optimizations
of your code
- __Consistency__: Iai-Callgrind can take accurate measurements even in virtualized CI environments
- __Performance__: Since Iai-Callgrind only executes a benchmark once, it is typically a lot faster
to run than benchmarks measuring the execution and wall time
- __Regression__: Iai-Callgrind reports the difference between benchmark runs to make it easy to
spot detailed performance regressions and improvements.
- __Profiling__: Iai-Callgrind generates a Callgrind profile of your code while benchmarking, so you
can use Callgrind-compatible tools like
[callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the results
in detail
- __Stable-compatible__: Benchmark your code without installing nightly Rust

### Update notes

Breaking change in `0.4.0`: In an effort to create more accurate event counts, the way of counting
events changed and may produce different results. See also [CHANGELOG](CHANGELOG.md)

### Installation

In order to use Iai-Callgrind, you must have [Valgrind](https://www.valgrind.org) installed. This
means that Iai-Callgrind cannot be used on platforms that are not supported by Valgrind.

To start with Iai-Callgrind, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
iai-callgrind = "0.4.0"
```

To be able to run the benchmarks you'll also need the `iai-callgrind-runner` binary installed
somewhere in your `$PATH`, for example with

```shell
cargo install --version 0.4.0 iai-callgrind-runner
```

There's also the possibility to install the binary somewhere else and point the
`IAI_CALLGRIND_RUNNER` environment variable to the absolute path of the `iai-callgrind-runner`
binary like so:

```shell
cargo install --version 0.4.0 --root /tmp iai-callgrind-runner
IAI_CALLGRIND_RUNNER=/tmp/bin/iai-callgrind-runner cargo bench --bench my-bench
```

When updating the `iai-callgrind` library, you'll also need to update `iai-callgrind-runner` and
vice-versa or else the benchmark runner will exit with an error.

### Benchmarking

`iai-callgrind` can be used to benchmark libraries or binaries. Library benchmarks benchmark
functions and methods of a crate and binary benchmarks benchmark the executables of a crate. The
different benchmark types cannot be intermixed in the same benchmark file but having different
benchmark files for library and binary benchmarks is no problem. More on that in the following
sections. For a quickstart and examples of benchmarking libraries see the [Library Benchmark
Section](#library-benchmarks) and for executables see the [Binary Benchmark
Section](#binary-benchmarks).

#### Library Benchmarks

Use this scheme if you want to micro-benchmark specific functions of your crate's library.

##### Quickstart

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

#[inline(never)] // required for benchmarking functions
fn iai_benchmark_short() -> u64 {
    fibonacci(black_box(10))
}

#[inline(never)] // required for benchmarking functions
fn iai_benchmark_long() -> u64 {
    fibonacci(black_box(30))
}

main!(iai_benchmark_short, iai_benchmark_long);
```

Note that it is important to annotate the benchmark functions with `#[inline(never)]` or else the
rust compiler will most likely try to optimize this function and inline it. `Callgrind` is function
(name) based and uses function calls within the benchmarking function to collect counter events. Not
inlining this function serves the additional purpose to reduce influences of the surrounding code on
the benchmark function.

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

##### Examples

For examples see also the [benches](iai-callgrind-runner/benches) folder.

###### Skipping setup code

Usually, all function calls in the benchmark function itself are attributed to the event counts. It's
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

### Binary Benchmarks

Use this scheme to benchmark one or more binaries of your crate. If you really like to it's possible
to benchmark any executable file in the `PATH` or any executable specified with an absolute path.
This may be useful if you want to compare the runtime of your crate with an existing tool.

It's also possible to run functions of the benchmark file before and after all benchmarks or to
setup and teardown any benchmarked binary.

#### Temporary Workspace

For security purposes, all binary benchmarks and the `before`, `after`, `setup` and `teardown`
functions are executed in a temporary directory. This directory will be created before the
[`before`](#before-after-setup-teardown-optional) function is run and removed after the `after`
function has finished. The [`fixtures`](#fixtures-optional) argument let's you copy your fixtures
into that directory, so you have access to all fixtures. However, if you want to access other
directories within the benchmarked package's directory, you need to specify absolute paths.

This probably sounds a bit weird at first, but another reason for using a temporary directory as
workspace is, that the length of the path where a benchmark is executed may have an influence on the
benchmark results. For example, running the benchmark in your repository `/home/me/my/repository`
and someone else's repository located under `/home/someone/else/repository` may produce different
results only because the length of the first path is shorter. To run benchmarks as deterministic as
possible across different systems, the length of the path should be the same wherever the benchmark
is executed. This crate ensures this property by using the `tempfile` crate which creates the
temporary directory in `/tmp` with a random name like `/tmp/.tmp12345678`. This ensures that the
length of the directory will be the same on all unix hosts where the benchmarks are run.

For the very same reasons, the environment variables of benchmarked binaries are cleared before the
benchmark is run. See also [`opts`](#options-optional).

#### Quickstart

Assuming the name of the crate's binary is `benchmark-tests`, add

```toml
[[bench]]
name = "my_binary_benchmark"
harness = false
```

to your `Cargo.toml` file and then create a file with the same `name` in
`benches/my_binary_benchmark.rs` with the following content:

```rust
use iai_callgrind::main;

/// This method is run before a benchmark
#[inline(never)] // required
fn setup() {
    println!("setup benchmark-tests")
}

/// This method is run after a benchmark
#[inline(never)] // required
fn teardown() {
    println!("teardown benchmark-tests");
}

main!(
    setup = setup;
    teardown = teardown;
    run = cmd = "benchmark-tests", args = ["one", "two"];
);
```

You're ready to run the benchmark with `cargo bench --bench my_binary_benchmark`. The rest of the
procedure is the same as with [Library Benchmarks](#library-benchmarks).

#### Description

- [Binary Benchmark Arguments](#description)
    - [run](#run-mandatory)
        - [cmd](#cmd-mandatory)
        - [args](#args-mandatory)
        - [opts](#opts-optional)
        - [envs](#envs-optional)
    - [options](#options-optional)
    - [before, after, setup, teardown](#before-after-setup-teardown-optional)
    - [fixtures](#fixtures-optional)

The `main` macro for binary benchmarks allows the following top-level arguments:

```rust
main!(
    options = "--callgrind-argument=yes";
    before = function_running_before_all_benchmarks;
    after = function_running_after_all_benchmarks;
    setup = function_running_before_any_benchmark;
    teardown = function_running_after_any_benchmark;
    fixtures = "path/to/fixtures";
    run = cmd = "benchmark-tests", args = [];
)
```

Here, `benchmark-tests` is an example of the name of the binary of a crate and it is assumed that
the `function_running_before_all_benchmarks` ... functions are defined somewhere in the same file of
the `main` macro. All top-level arguments must be separated by a `;`. However, only `run` is
mandatory. All other top-level arguments (like `options`, `setup` etc.) are optional.

##### `run` (Mandatory)

The `run` argument can be specified multiple times separated by a `;` but must be given at least
once. It takes the following arguments:

###### `cmd` (Mandatory)

This argument is allowed only once and specifies the name of one of the executables of the
benchmarked crate. The path of the executable is discovered automatically, so the name of the
`[[bin]]` as specified in the crate's `Cargo.toml` file is sufficient. The auto discovery supports
running the benchmarks with different profiles.

Although not the main purpose of `iai-callgrind`, it's possible to benchmark any executable in the
`PATH` or specified with an absolute path.

###### `args` (Mandatory)

The `args` argument must be specified at least once containing the arguments for the benchmarked
`cmd`. It can be an empty array `[]` to run to the [`cmd`](#cmd-mandatory) without any arguments.
Specifying `args` multiple times (separated by a `,`)

```rust
main!(
    run = cmd = "benchmark-tests", args = ["something"], args = ["other"]
)
```

is a short-hand for specifying [`run`](#run-mandatory) with the same [`cmd`](#cmd-mandatory),
[`opts`](#opts-optional) and [`envs`](#envs-optional) arguments multiple times

```rust
main!(
    run = cmd = "benchmark-tests", args = ["something"];
    run = cmd = "benchmark-tests", args = ["other"]
)
```

###### `opts` (Optional)

`opts` is optional and can be specified once for every `run` and [`cmd`](#cmd-mandatory):

```rust
main!(
    run = cmd = "benchmark-tests",
        opts = Options::default().env_clear(false),
        args = ["something"];
)
```

Here, `env_clear(false)` specifies to keep the environment variables when running the `cmd` with
`callgrind`.

The currently available options are:

- `env_clear`: If `true` clear the environment variables before running the benchmark (Default: `true`)
- `current_dir`: Set the working directory of the `cmd` (Default: Unchanged)
- `entry_point`: Per default the counting of events starts right at the start of the binary and
stops when it finished execution. It may desirable to start the counting for example when entering
the `main` function (but can be any function) and stop counting when leaving the `main` function of
the executable. The `entry_point` could look like `benchmark_tests::main` for a binary with the name
`benchmark-tests` (Note that hyphens are replaced with an underscore by `callgrind`). See also the
documentation of
[toggle-collect](https://valgrind.org/docs/manual/cl-manual.html#opt.toggle-collect) and
[Limiting the range of collected
events](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.limits)

###### `envs` (Optional)

`envs` may be used to set environment variables available in the `cmd`. This argument is optional
and can be specified once for every [`cmd`](#cmd-mandatory). There must be at least one `KEY=VALUE`
pair or `KEY` present in the array:

```rust
main!(
    run = cmd = "benchmark-tests",
        envs = ["MY_VAR=SOME_VALUE", "MY_OTHER_VAR=VALUE"],
        args = ["something"];
)
```

Environment variables specified in the `envs` array are usually `KEY=VALUE` pairs. But, if
`env_clear` is true (what is the default), single `KEY`s are environment variables to pass-through
to the `cmd`. The following will pass-through the `PATH` variable although the environment is
cleared (here given explicitly with the `Options` although it is the default)

```rust
main!(
    run = cmd = "benchmark-tests",
        envs = ["PATH"],
        opts = Options::default().env_clear(true),
        args = [];
)
```

Pass-through environment variables are ignored if they don't exist in the root environment.

##### `options` (Optional)

A `,` separated list of strings which contain options for all `callgrind` invocations and therefore
benchmarked `cmd`s (Including benchmarked `before`, `after`, `setup` and `teardown` functions).

```rust
main!(
    options = "--zero-before=benchmark_tests::main";
    run = cmd = "benchmark-tests", args = [];
)
```

See also [Passing arguments to callgrind](#passing-arguments-to-callgrind) and the documentation of
[Callgrind](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)

##### `before`, `after`, `setup`, `teardown` (Optional)

Each of the `before`, `after`, `setup` and `teardown` top-level arguments is optional. If given,
this argument must specify a function of the benchmark file. These functions are meant to setup and
cleanup the benchmarks. Each function is invoked at a different stage of the benchmarking process.

- `before`: This function is run once before all benchmarked `cmd`s
- `after`: This function is run once after all benchmarked `cmd`s
- `setup`: This function is run once before any benchmarked `cmd`
- `teardown`: This function is run once after any benchmarked `cmd`

```rust
use iai_callgrind::main;

#[inline(never)] // necessary
fn setup_my_benchmark() {
    // For example, create a file
}

#[inline(never)] // necessary
fn teardown_my_benchmark() {
    // For example, delete a file
}

main!(
    setup = setup_my_benchmark;
    teardown = teardown_my_benchmark;
    run = cmd = "benchmark-tests", args = [];
)
```

Per default, these functions are not benchmarked, but this behavior can be changed by specifying the
optional `bench` argument with a value of `true` after the function name.

```rust
main!(
    setup = setup_my_benchmark, bench = true;
    run = cmd = "benchmark-tests", args = [];
)
```

Note that `setup` and `teardown` functions are benchmarked only once the first time they are
invoked, much like the `before` and `after` functions. However, these functions are run as usual
before or after any benchmark. Benchmarked `before`, `after` etc. functions follow the same rules as
benchmark functions of [library benchmarks](#library-benchmarks).

##### `fixtures` (Optional)

The `fixtures` argument specifies a path to a directory containing fixtures which you want to be
available for all benchmarks and the `before`, `after`, `setup` and `teardown` functions. The
fixtures directory will be copied as is into the workspace directory of the benchmark. Relative
paths are interpreted relative to the benchmarked package. In a multi-package workspace this'll be
the package name of the benchmark. Otherwise, it'll be the workspace root.

```rust
main!(
    setup = setup_my_benchmark;
    fixtures = "my_fixtures";
    run = cmd = "benchmark-tests", args = [];
)
```

Here, the directory `my_fixtures` in the root of the package under test will be copied into the
[temporary workspace](#temporary-workspace) (for example `/tmp/.tmp12345678`). So, the setup
function `setup_my_benchmark` and the benchmark of `benchmarks-tests` can access a fixture
`test_1.txt` with a relative path like `my_fixtures/test_1.txt`

#### Examples

See the [test_bin_bench](benchmark-tests/benches/test_bin_bench.rs) benchmark file of this project for an example.

### Features and differences to Iai

This crate is built on the same idea like the original Iai, but over the time applied a lot of
improvements. The biggest difference is, that it uses Callgrind under the hood instead of
Cachegrind.

#### More stable metrics

Iai-Callgrind has even more precise and stable metrics across different systems. It achieves this by

- only counting events of function calls within the benchmarking function. This behavior virtually
encapsulates the benchmark function and separates the benchmark from the surrounding code.
- separating the iai library with the main macro from the actual runner. This is the reason for the
extra installation step of `iai-callgrind-runner` but before this separation even small changes in
the iai library had effects on the benchmarks under test.

Below a local run of one of the benchmarks of this library

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

Iai-Callgrind cannot completely remove the influences of setup changes. However, these effects
shouldn't be significant anymore.

### See also

- The user guide of the original Iai: <https://bheisler.github.io/criterion.rs/book/iai/iai.html>
- A comparison of criterion-rs with Iai: <https://github.com/bheisler/iai#comparison-with-criterion-rs>

### Credits

Iai-Callgrind is forked from <https://github.com/bheisler/iai> and was originally written by Brook
Heisler (@bheisler).

### License

Iai-Callgrind is like Iai dual licensed under the Apache 2.0 license and the MIT license.
