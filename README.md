<!-- spell-checker: ignore fixt binstall libtest eprintln usize -->

<h1 align="center">Iai-Callgrind</h1>

<div align="center">High-precision and consistent benchmarking framework/harness for Rust</div>

<div align="center">
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
        <img src="https://img.shields.io/badge/MSRV-1.66.0-brightgreen" alt="MSRV"/>
    </a>
</div>

Iai-Callgrind is a benchmarking framework/harness which primarily uses
[Valgrind's Callgrind](https://valgrind.org/docs/manual/cl-manual.html) and the
other Valgrind tools to provide extremely accurate and consistent measurements
of Rust code, making it perfectly suited to run in environments like a CI.

This crate started as a fork of the great [Iai](https://github.com/bheisler/iai) crate rewritten to
use Valgrind's [Callgrind](https://valgrind.org/docs/manual/cl-manual.html) instead of
[Cachegrind](https://valgrind.org/docs/manual/cg-manual.html) but also adds a lot of other
improvements and features.

## Table of Contents

- [Table of Contents](#table-of-contents)
    - [Features](#features)
    - [Installation](#installation)
    - [Benchmarking](#benchmarking)
        - [Library Benchmarks](#library-benchmarks)
        - [Binary Benchmarks](#binary-benchmarks)
    - [Performance Regressions](#performance-regressions)
    - [Valgrind Tools](#valgrind-tools)
    - [Valgrind Client Requests](#valgrind-client-requests)
    - [Flamegraphs](#flamegraphs)
    - [Command-line arguments and environment variables](#command-line-arguments-and-environment-variables)
        - [Baselines](#comparing-with-baselines)
        - [Output directory](#customize-the-output-directory)
        - [Machine-readable output](#machine-readable-output)
        - [Other output options](#other-output-options)
    - [Features and differences to Iai](#features-and-differences-to-iai)
    - [FAQ](#faq)
    - [What hasn't changed](#what-hasnt-changed)
    - [See also](#see-also)
    - [Contributing](#contributing)
    - [Credits](#credits)
    - [License](#license)

### Features

- __Precision__: High-precision measurements allow you to reliably detect very
  small optimizations of your code
- __Consistency__: Iai-Callgrind can take accurate measurements even in
  virtualized CI environments
- __Performance__: Since Iai-Callgrind only executes a benchmark once, it is
  typically a lot faster to run than benchmarks measuring the execution and wall
  time
- __Regression__: Iai-Callgrind reports the difference between benchmark runs to
  make it easy to spot detailed performance regressions and improvements. You
  can define limits for specific event kinds to fail a benchmark if that limit
  is breached.
- __CPU and Cache Profiling__: Iai-Callgrind generates a Callgrind profile of
  your code while benchmarking, so you can use Callgrind-compatible tools like
  [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
  or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html)
  to analyze the results in detail.
- __Memory Profiling__: You can run other Valgrind tools like [DHAT: a dynamic
  heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html) and
  [Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html)
  with the Iai-Callgrind benchmarking framework. Their profiles are stored next
  to the callgrind profiles and are ready to be examined with analyzing tools
  like `dh_view.html`, `ms_print` and others.
- __Visualization__: Iai-Callgrind is capable of creating regular and
  differential flamegraphs from the Callgrind output format.
- __Valgrind Client Requests__: Support of zero overhead [Valgrind Client
  Requests](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)
  (compared to native valgrind client requests overhead) on many targets
- __Stable-compatible__: Benchmark your code without installing nightly Rust

### Installation

In order to use Iai-Callgrind, you must have [Valgrind](https://www.valgrind.org) installed. This
means that Iai-Callgrind cannot be used on platforms that are not supported by Valgrind.

To start with Iai-Callgrind, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
iai-callgrind = "0.12.0"
```

To be able to run the benchmarks you'll also need the `iai-callgrind-runner` binary installed
somewhere in your `$PATH`, for example with

```shell
cargo install --version 0.12.0 iai-callgrind-runner
```

or with `binstall`

```shell
cargo binstall iai-callgrind-runner@0.12.0
```

There's also the possibility to install the binary somewhere else and point the
`IAI_CALLGRIND_RUNNER` environment variable to the absolute path of the `iai-callgrind-runner`
binary like so:

```shell
cargo install --version 0.12.0 --root /tmp iai-callgrind-runner
IAI_CALLGRIND_RUNNER=/tmp/bin/iai-callgrind-runner cargo bench --bench my-bench
```

When updating the `iai-callgrind` library, you'll also need to update
`iai-callgrind-runner` and vice-versa or else the benchmark runner will exit
with an error. Otherwise, there is no need to interact with
`iai-callgrind-runner` as it is just an implementation detail.

Since the `iai-callgrind-runner` version must match the `iai-callgrind` library
version it's best to automate this step in the CI. A job step in the github
actions ci could look like this

```yaml
- name: Install iai-callgrind-runner
  run: |
    version=$(cargo metadata --format-version=1 |\
      jq '.packages[] | select(.name == "iai-callgrind").version' |\
      tr -d '"'
    )
    cargo install iai-callgrind-runner --version $version
```

If you want to make use of the [Valgrind Client
Requests](#valgrind-client-requests) you need `libclang` (clang >= 5.0)
installed. See also the requirements of
[bindgen](https://rust-lang.github.io/rust-bindgen/requirements.html)) and of
[cc](https://github.com/rust-lang/cc-rs).

`iai-callgrind` needs the debug symbols when running the benchmarks. There are
multiple places where you can configure profiles, see the
[Benchmarking](#benchmarking) section below for more details.

### Benchmarking

`iai-callgrind` can be used to benchmark libraries or binaries. Library benchmarks benchmark
functions and methods of a crate and binary benchmarks benchmark the executables of a crate. The
different benchmark types cannot be intermixed in the same benchmark file but having different
benchmark files for library and binary benchmarks is no problem. More on that in the following
sections.

For a quickstart and examples of benchmarking libraries see the [Library Benchmark
Section](#library-benchmarks) and for executables see the [Binary Benchmark
Section](#binary-benchmarks). Read the [docs]!

As mentioned in above in the `Installation` section, it's required to run the
benchmarks with debugging symbols switched on. For example in your
`~/.cargo/config` or your project's `Cargo.toml`:

```toml
[profile.bench]
debug = true
```

Now, all benchmarks you run with `cargo bench` include the debug symbols. (See also
[Cargo Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and
[Cargo Config](https://doc.rust-lang.org/cargo/reference/config.html)).

It's required that settings like `strip = true` or other configuration options
stripping the debug symbols need to be disabled explicitly for the `bench`
profile if you have changed this option for the `release` profile. For example:

```toml
[profile.release]
strip = true

[profile.bench]
debug = true
strip = false
```

Per default, `iai-callgrind` runs all benchmarks with Valgrind's cache
simulation turned on (`--cache-sim=yes`) in order to calculate an estimation for
the total cpu cycles. See also the [Metrics Output](#rework-the-metrics-output)
section for more infos. However, if you want to opt-out of the cache simulation
and the calculation of estimated cycles, you can easily do so within the
benchmark with the `LibraryBenchmarkConfig` (or `BinaryBenchmarkConfig`):

```rust
LibraryBenchmarkConfig::default().raw_callgrind_args(["--cache-sim=no"])
```

in the cli with environment variables

```shell
IAI_CALLGRIND_CALLGRIND_ARGS="--cache-sim=no" cargo bench
```

or with arguments

```shell
cargo bench -- --callgrind-args="--cache-sim=no"
```

To be able to run the latter command, some [additional
configuration](#running-cargo-bench-results-in-an-unrecognized-option-error)
might be needed.

#### Library Benchmarks

Use this scheme if you want to micro-benchmark specific functions of your crate's library.

#### Important default behavior

The environment variables are cleared before running a library benchmark. Have a
look into the [Configuration](#configuration) section if you need to change that
behavior.

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

main!(library_benchmark_groups = bench_fibonacci_group);
```

Note that it is important to annotate the benchmark functions with `#[library_benchmark]`. But,
there's no need to annotate benchmark functions with `inline(never)` anymore. The `bench` attribute
takes any expression what includes function calls. The following would have worked too and avoids
setup code within the benchmark function eliminating the need to pass `toggle-collect` arguments to
callgrind:

```rust
fn some_setup_func(value: u64) -> u64 {
    value
}

#[library_benchmark]
#[bench::long(some_setup_func(30))]
fn bench_fibonacci(value: u64) -> u64 {
    black_box(fibonacci(value))
}
```

Now, you can run this benchmark with `cargo bench --bench my_benchmark` in your project root and you
should see something like this:

```text
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci short:10
  Instructions:                1733|N/A             (*********)
  L1 Hits:                     2359|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       2|N/A             (*********)
  Total read+write:            2361|N/A             (*********)
  Estimated Cycles:            2429|N/A             (*********)
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci long:30
  Instructions:            26214733|N/A             (*********)
  L1 Hits:                 35638617|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       4|N/A             (*********)
  Total read+write:        35638621|N/A             (*********)
  Estimated Cycles:        35638757|N/A             (*********)
```

In addition, you'll find the callgrind output in `target/iai`, if you want to investigate further
with a tool like `callgrind_annotate`. When running the same benchmark again, the output will
report the differences between the current and the previous run. Say you've made change to the
`fibonacci` function, then you may see something like this:

```text
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci short:10
  Instructions:                2804|1733            (+61.8003%) [+1.61800x]
  L1 Hits:                     3815|2359            (+61.7211%) [+1.61721x]
  L2 Hits:                        0|0               (No change)
  RAM Hits:                       2|2               (No change)
  Total read+write:            3817|2361            (+61.6688%) [+1.61669x]
  Estimated Cycles:            3885|2429            (+59.9424%) [+1.59942x]
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci long:30
  Instructions:            16201596|26214733        (-38.1966%) [-1.61803x]
  L1 Hits:                 22025878|35638617        (-38.1966%) [-1.61803x]
  L2 Hits:                        0|0               (No change)
  RAM Hits:                       3|4               (-25.0000%) [-1.33333x]
  Total read+write:        22025881|35638621        (-38.1966%) [-1.61803x]
  Estimated Cycles:        22025983|35638757        (-38.1965%) [-1.61803x]
```

##### The #[library_benchmark] attribute in more detail

This attribute needs to be present on all benchmark functions specified in the
`library_benchmark_group`. The benchmark function can then be further annotated
with the `#[bench]` or `#[benches]` attributes.

```rust
#[library_benchmark]
#[bench::first(21)]
fn my_bench(value: u64) -> u64 {
    // benchmark something
}

library_benchmark_group!(name = my_group; benchmarks = my_bench);
```

The following parameters are accepted:

- `config`: Accepts a `LibraryBenchmarkConfig`
- `setup`: A global setup function which is applied to all following `#[bench]`
  and `#[benches]` attributes if not overwritten by a `setup` parameter of these
  attributes.
- `teardown`: Similar to `setup` but takes a global `teardown` function.

A short example on the usage of the `setup` parameter:

```rust
fn my_setup(value: u64) -> String {
     format!("{value}")
}

fn my_other_setup(value: u64) -> String {
     format!("{}", value + 10)
}

#[library_benchmark(setup = my_setup)]
#[bench::first(21)]
#[benches::multiple(42, 84)]
#[bench::last(args = (102), setup = my_other_setup)]
fn my_bench(value: String) {
    println!("{value}");
}
```

Here, the benchmarks with the id `first` and `multiple` use the `my_setup`
function, and `last` uses `my_other_setup`.

And a short example of the `teardown` parameter:

```rust
fn my_teardown(value: usize) {
     println!("The length of the input string was: {value}");
}

fn my_other_teardown(value: usize) {
     if value != 3 {
         panic!("The length of the input string was: {value} but expected it to be 3");
     }
}

#[library_benchmark(teardown = my_teardown)]
#[bench::first("1")]
#[benches::multiple("42", "84")]
#[bench::last(args = ("104"), teardown = my_other_teardown)]
fn my_bench(value: &str) -> usize {
    // Let's benchmark the `len` function
    black_box(value.len())
}
```

This example works well with the `--nocapture` option (env: `IAI_CALLGRIND_NOCAPTURE`,
see also [Show terminal output of
benchmarks](#show-terminal-output-of-benchmarks)), so you can actually see
the output of the `my_teardown` function.

##### The #[bench] attribute

The basic structure is `#[bench::some_id(/* parameters */)]`. The part after the
`::` must be an id unique within the same `#[library_benchmark]`. This attribute
accepts the following parameters:

- `args`: A tuple with a list of arguments which are passed to the
  benchmark function. The parentheses also need to be present if there is only a
  single argument (`#[bench::my_id(args = (10))]`).
- `config`: Accepts a `LibraryBenchmarkConfig`
- `setup`: A function which takes the arguments specified in the `args`
  parameter and passes its return value to the benchmark function.
- `teardown`: A function which takes the return value of the benchmark function.

If no other parameters besides `args` are present you can simply pass the
arguments as a list of values. Instead of `#[bench::my_id(args = (10, 20))]`,
you could also use the shorter `#[bench::my_id(10, 20)]`.

##### Specify multiple benchmarks at once with the #[benches] attribute

This attribute accepts the same parameters as the `#[bench]` attribute: `args`,
`config`, `setup` and `teardown` and additionally the `file` parameter. In
contrast to the `args` parameter in `#[bench]`, `args` takes an array of
arguments.

Let's start with an example:

```rust
fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[benches::multiple(vec![1], vec![5])]
#[benches::with_setup(args = [1, 5], setup = setup_worst_case_array)]
fn bench_bubble_sort_with_benches_attribute(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}
```

Usually the `arguments` are passed directly to the benchmarking function as it
can be seen in the `#[benches::multiple(...)]` case. In
`#[benches::with_setup(...)]`, the arguments are passed to the `setup` function
instead. The above `#[library_benchmark]` is pretty much the same as

```rust
#[library_benchmark]
#[bench::multiple_0(vec![1])]
#[bench::multiple_1(vec![5])]
#[bench::with_setup_0(setup_worst_case_array(1)])]
#[bench::with_setup_1(setup_worst_case_array(5)])]
fn bench_bubble_sort_with_benches_attribute(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}
```

but a lot more concise especially if a lot of values are passed to the same
`setup` function.

The `file` parameter goes a step further and reads the specified file line by
line creating a benchmark out of each line. The line is passed to the benchmark
function as `String` or if the `setup` parameter is also present to the `setup`
function. A small example assuming you have a file `benches/inputs` with the
following content

```text
1
11
111
```

then

```rust
#[library_benchmark]
#[benches::by_file(file = "benches/inputs")]
fn some_bench(line: String) -> Result<u64> {
    black_box(my_lib::string_to_u64(line))
}
```

The above is roughly equivalent to the following but with the `args` parameter

```rust
#[library_benchmark]
#[benches::by_file(args = [1.to_string(), 11.to_string(), 111.to_string()])]
fn some_bench(line: String) -> Result<u64> {
    black_box(my_lib::string_to_u64(line))
}
```

Reading inputs from a file allows for example sharing the same inputs between
different benchmarking frameworks like `criterion` or if you simply have a long
list of inputs you might find it more convenient to read them from a file.

##### Comparing benchmark functions

Comparing benchmark functions is supported via the optional
`library_benchmark_group!` argument `compare_by_id` (The default value for
`compare_by_id` is `false`). Only benches with the same `id` are compared, which
allows to single out cases which don't need to be compared. In the following
example, the `case_3` and `multiple` bench are compared with each other in
addition to the usual comparison with the previous run:

```rust
#[library_benchmark]
#[bench::case_3(vec![1, 2, 3])]
#[benches::multiple(args = [vec![1, 2], vec![1, 2, 3, 4]])]
fn bench_bubble_sort_best_case(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}

#[library_benchmark]
#[bench::case_3(vec![3, 2, 1])]
#[benches::multiple(args = [vec![2, 1], vec![4, 3, 2, 1]])]
fn bench_bubble_sort_worst_case(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}

library_benchmark_group!(
    name = bench_bubble_sort;
    compare_by_id = true;
    benchmarks = bench_bubble_sort_best_case, bench_bubble_sort_worst_case
);
```

Note if `compare_by_id` is `true`, all benchmark functions are compared with
each other, so you are not limited to two benchmark functions per comparison
group.

Here's a curated excerpt from the output of the above example to see what is
happening:

```text
test_lib_bench_compare::bubble_sort_compare::bench_bubble_sort_best_case case_3:vec! [1, 2, 3]
  Instructions:                  94|N/A             (*********)
  L1 Hits:                      124|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       4|N/A             (*********)
  Total read+write:             128|N/A             (*********)
  Estimated Cycles:             264|N/A             (*********)
test_lib_bench_compare::bubble_sort_compare::bench_bubble_sort_worst_case case_3:vec! [3, 2, 1]
  Instructions:                 103|N/A             (*********)
  L1 Hits:                      138|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       5|N/A             (*********)
  Total read+write:             143|N/A             (*********)
  Estimated Cycles:             313|N/A             (*********)
  Comparison with bench_bubble_sort_best_case case_3:vec! [1, 2, 3]
  Instructions:                  94|103             (-8.73786%) [-1.09574x]
  L1 Hits:                      124|138             (-10.1449%) [-1.11290x]
  L2 Hits:                        0|0               (No change)
  RAM Hits:                       4|5               (-20.0000%) [-1.25000x]
  Total read+write:             128|143             (-10.4895%) [-1.11719x]
  Estimated Cycles:             264|313             (-15.6550%) [-1.18561x]
```

Here's the procedure of the comparison algorithm:

1. Run all benches in the first benchmark function
2. Run the first bench in the second benchmark function and if there is a bench
   in the first benchmark function with the same id compare them
3. Run the second bench in the second benchmark function ...
4. ...
5. Run the first bench in the third benchmark function and if there is a bench
   in the first benchmark function with the same id compare them. If there is a
   bench with the same id in the second benchmark function compare them.
6. Run the second bench in the third benchmark function ...
7. and so on ... until all benches are compared with each other

Neither the order nor the amount of benches within the benchmark functions
matters, so it is not strictly necessary to mirror the bench ids of the first
benchmark function in the second, third, etc. benchmark function.

##### Examples

For a fully documented and working benchmark see the
[test_lib_bench_groups](benchmark-tests/benches/test_lib_bench_groups.rs) benchmark file and read
the [`library documentation`]!

##### Configuration

It's possible to configure some of the behavior of `iai-callgrind`. See the [docs] of
`LibraryBenchmarkConfig` for more details. At top-level with the `main!` macro:

```rust
main!(
    config = LibraryBenchmarkConfig::default();
    library_benchmark_groups = ...
);
```

At group-level:

```rust
library_benchmark_groups!(
    name = some_name;
    config = LibraryBenchmarkConfig::default();
    benchmarks = ...
);
```

At `library_benchmark` level:

```rust
#[library_benchmark(config = LibraryBenchmarkConfig::default())]
...
```

and at `bench` level:

```rust
#[library_benchmark]
#[bench::some_id(args = (1, 2), config = LibraryBenchmarkConfig::default()]
...
```

The config at `bench` level overwrites the config at `library_benchmark` level. The config at
`library_benchmark` level overwrites the config at group level and so on. Note that configuration
values like `envs` are additive and don't overwrite configuration values of higher levels.

### Binary Benchmarks

Use this scheme to benchmark one or more binaries of your crate. If you really like to, it's
possible to benchmark any executable file in the `PATH` or any executable specified with an absolute
path.

It's also possible to run functions of the same benchmark file `before` and `after` all benchmarks
or to `setup` and `teardown` any benchmarked binary.

Unlike [Library Benchmarks](#library-benchmarks), there are no setup costs for binary benchmarks to
pay attention at, since each benchmark run's command is passed directly to valgrind's callgrind.

#### Temporary Workspace and other important default behavior

Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown` functions are
executed in a temporary directory. See the [Switching off the sandbox](#switching-off-the-sandbox)
for changing this behavior.

Also, the environment variables of benchmarked binaries are cleared before the benchmark is run. See
also [Environment variables](#environment-variables) for how to pass environment variables to the
benchmarked binary.

#### Quickstart

Suppose your crate's binary is named `benchmark-tests-printargs` and you have a
fixtures directory in `fixtures` with a file `test1.txt` in it:

```rust
use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, BinaryBenchmarkGroup,
    Fixtures, Run,
};

fn my_setup() {
    println!("We can put code in here which will be run before each benchmark run");
}

// We specify a cmd `"benchmark-tests-exe"` for the whole group which is a
// binary of our crate. This eliminates the need to specify a `cmd` for each
// `Run` later on and we can use the auto-discovery of a crate's binary at group
// level. We'll also use the `setup` argument to run a function before each of
// the benchmark runs.
binary_benchmark_group!(
    name = my_exe_group;
    setup = my_setup;
    // This directory will be copied into the root of the sandbox (as `fixtures`)
    config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("fixtures"));
    benchmark =
        |"benchmark-tests-printargs", group: &mut BinaryBenchmarkGroup| {
            setup_my_exe_group(group)
    }
);

// Working within a macro can be tedious sometimes so we moved the setup code
// into this method
fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
    group
        // Setup our first run doing something with our fixture `test1.txt`. The
        // id (here `do foo with test1`) of an `Arg` has to be unique within the
        // same group
        .bench(Run::with_arg(Arg::new(
            "do foo with test1",
            ["--foo=fixtures/test1.txt"],
        )))

        // Setup our second run with two positional arguments. We're not
        // interested in anything happening before the main function in
        // `benchmark-tests-printargs`, so we set the entry_point.
        .bench(
            Run::with_arg(
                Arg::new(
                    "positional arguments",
                    ["foo", "foo bar"],
                )
            ).entry_point("benchmark_tests_printargs::main")
        )

        // Our last run doesn't take an argument at all.
        .bench(Run::with_arg(Arg::empty("no argument")));
}

// As last step specify all groups we want to benchmark in the main! macro
// argument `binary_benchmark_groups`. The main macro is always needed and
// finally expands to a benchmarking harness
main!(binary_benchmark_groups = my_exe_group);
```

You're ready to run the benchmark with `cargo bench --bench my_binary_benchmark`.

The output of this benchmark run could look like this:

```text
my_binary_benchmark::my_exe_group do foo with test1:benchmark-tests-printargs "--foo=fixt...
  Instructions:              331082|N/A             (*********)
  L1 Hits:                   442452|N/A             (*********)
  L2 Hits:                      720|N/A             (*********)
  RAM Hits:                    3926|N/A             (*********)
  Total read+write:          447098|N/A             (*********)
  Estimated Cycles:          583462|N/A             (*********)
my_binary_benchmark::my_exe_group positional arguments:benchmark-tests-printargs foo "foo ba...
  Instructions:                3906|N/A             (*********)
  L1 Hits:                     5404|N/A             (*********)
  L2 Hits:                        8|N/A             (*********)
  RAM Hits:                      91|N/A             (*********)
  Total read+write:            5503|N/A             (*********)
  Estimated Cycles:            8629|N/A             (*********)
my_binary_benchmark::my_exe_group no argument:benchmark-tests-printargs
  Instructions:              330070|N/A             (*********)
  L1 Hits:                   441031|N/A             (*********)
  L2 Hits:                      716|N/A             (*********)
  RAM Hits:                    3925|N/A             (*********)
  Total read+write:          445672|N/A             (*********)
  Estimated Cycles:          581986|N/A             (*********)
```

You'll find the callgrind output files of each run of the benchmark
`my_binary_benchmark` of the group `my_exe_group` in
`target/iai/$CARGO_PKG_NAME/my_binary_benchmark/my_exe_group`. See also
[customizing the output directory](#customize-the-output-directory).

#### Configuration

Much like the configuration of [Library Benchmarks](#configuration) it's possible to configure
binary benchmarks at top-level in the `main!` macro and at group-level in the
`binary_benchmark_groups!` with the `config = ...;` argument. In contrast to library benchmarks,
binary benchmarks can be configured at a lower and last level within `Run` directly.

#### Auto-discovery of a crate's binaries

Auto-discovery of a crate's binary works only when specifying the name of it at group level.

```rust
binary_benchmark_group!(
    name = my_exe_group;
    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {});
```

If you don't like specifying a default command at group level, you can use
`env!("CARGO_BIN_EXE_name)` at `Run`-level like so:

```rust
binary_benchmark_group!(
    name = my_exe_group;
    benchmark = |group: &mut BinaryBenchmarkGroup| {
        group.bench(Run::with_cmd(env!("CARGO_BIN_EXE_my-exe"), Arg::empty("some id")));
    });
```

#### A benchmark run of a binary exits with error

Usually, if a benchmark exits with a non-zero exit code, the whole benchmark run fails and stops.
If you expect the exit code of your benchmarked binary to be different from `0`, you can set the
expected exit code with `Options` at `Run`-level

```rust
binary_benchmark_group!(
    name = my_exe_group;
    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
        group.bench(
            Run::with_arg(
                Arg::empty("some id")
            )
            .options(Options::default().exit_with(ExitWith::Code(100)))
        );
    });
```

#### Environment variables

Per default, the environment variables are cleared before running a benchmark.

It's possible to specify environment variables at `Run`-level which should be available in the
binary:

```rust
binary_benchmark_group!(
    name = my_exe_group;
    benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| {
        group.bench(Run::with_arg(Arg::empty("some id")).envs(["KEY=VALUE", "KEY"]));
    });
```

Environment variables specified in the `envs` array are usually `KEY=VALUE` pairs. But, if
`env_clear` is true (what is the default), single `KEY`s are environment variables to pass-through
to the `cmd`. Pass-through environment variables are ignored if they don't exist in the root
environment.

#### Switching off the sandbox

Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown` functions are
executed in a temporary directory. This behavior can be switched off at group-level:

```rust
binary_benchmark_group!(
    name = my_exe_group;
    benchmark = |group: &mut BinaryBenchmarkGroup| {
        group.sandbox(false);
    });
```

#### Examples

See the [test_bin_bench_groups](benchmark-tests/benches/test_bin_bench_groups.rs) benchmark file of
this project for a working example.

### Performance Regressions

With Iai-Callgrind you can define limits for each event kinds over which a
performance regression can be assumed. There are no default regression checks
and you have to opt-in with a `RegressionConfig` at benchmark level or at a
global level with [Command-line arguments or Environment
variables](#command-line-arguments-and-environment-variables).

A performance regression check consists of an `EventKind` and a percentage over
which a regression is assumed. If the percentage is negative, then a regression
is assumed to be below this limit. The default `EventKind` is
`EventKind::Ir` with a value of `+10%`.For example, in a [Library
Benchmark](#library-benchmarks), let's overwrite the default limit with a global
limit of `+5%` for the total instructions executed (the `Ir` event kind):

```rust
main!(
    config = LibraryBenchmarkConfig::default()
        .regression(
            RegressionConfig::default()
                .limits([(EventKind::Ir, 5.0)])
        );
    library_benchmark_groups = some_group
);
```

For example [SQLite](https://sqlite.org/cpu.html#performance_measurement) uses
mainly cpu instructions to measure performance improvements (and regressions).

For more details on regression checks consult the iai-callgrind [docs].

### Valgrind Tools

In addition to the default benchmarks, you can use the Iai-Callgrind framework
to run other Valgrind profiling `Tool`s like `DHAT`, `Massif` and the
experimental `BBV` but also `Memcheck`, `Helgrind` and `DRD` if you need to
check memory and thread safety of benchmarked code. See also the [Valgrind User
Manual](https://valgrind.org/docs/manual/manual.html) for more details and
command line arguments. The additional tools can be specified in
`LibraryBenchmarkConfig`, `BinaryBenchmarkConfig` or `Run`. For example to run
`DHAT` for all library benchmarks:

```rust
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig, Tool,
    ValgrindTool
};

#[library_benchmark]
fn some_func() {
    println!("Hello, World!");
}

library_benchmark_group!(name = some_group; benchmarks = some_func);

main!(
    config = LibraryBenchmarkConfig::default()
                .tool(Tool::new(ValgrindTool::DHAT));
    library_benchmark_groups = some_group
);
```

All tools which produce an `ERROR SUMMARY` `(Memcheck, DRD, Helgrind)` have
`--error-exitcode=201` ([See
also](https://valgrind.org/docs/manual/manual-core.html#manual-core.erropts))
set, so if there are any errors, the benchmark run fails with `201`. You can
overwrite this default with

```rust
Tool::new(ValgrindTool::Memcheck).args("--error-exitcode=0")
```

which would restore the default of `0` from valgrind.

### Valgrind Client Requests

`iai-callgrind` ships with it's own interface to [Valgrind's Client Request
Mechanism](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq).
`iai-callgrind's` client requests have (compared to the valgrind's client
requests used in `C` code) zero overhead on many targets which are also natively
supported by valgrind. My opinion may be biased, but compared to other crates
that offer an interface to valgrind's client requests, `iai-callgrind` provides
the most complete and best performant implementation.

Client requests are deactivated by default but can be activated with the
`client_requests` feature.

```toml
[dev-dependencies]
iai-callgrind = { version = "0.12.0", features = ["client_requests"] }
```

If you need the client requests in your production code, you usually don't want
them to do anything when not running under valgrind with `iai-callgrind`
benchmarks. You can achieve that by adding `iai-callgrind` with the
`client_requests_defs` feature to your runtime dependencies and with the
`client_requests` feature to your `dev-dependencies` like so:

```toml
[dependencies]
iai-callgrind = { version = "0.12.0", default-features = false, features = [
    "client_requests_defs"
] }

[dev-dependencies]
iai-callgrind = { version = "0.12.0", features = ["client_requests"] }
```

With just the `client_requests_defs` feature activated, the client requests
compile down to nothing and don't add any overhead to your production code. It
simply provides the "definitions", method signatures and macros without body.
Only with the activated `client_requests` feature they will be actually
executed. Note that the client requests do not depend on any other part of
`iai-callgrind`, so you could even use the client requests without the rest of
`iai-callgrind`.

Use them in your code for example like so:

```rust
use iai_callgrind::client_requests;

fn main() {
    // Start callgrind event counting if not already started earlier
    client_requests::callgrind::start_instrumentation();

    // do something important

    // Toggle callgrind event counting off
    client_requests::callgrind::toggle_collect();
}
```

When building `iai-callgrind` with client requests, the valgrind header files
must exist in your standard include path (most of the time `/usr/include`). This
is usually the case if you've installed valgrind with your distribution's
package manager. If not, you can point the `IAI_CALLGRIND_VALGRIND_INCLUDE` or
`IAI_CALLGRIND_<triple>_VALGRIND_INCLUDE` environment variables to the include
path. So, if the headers can be found in
`/home/foo/repo/valgrind/{valgrind.h, callgrind.h, ...}`, the correct include
path would be `IAI_CALLGRIND_VALGRIND_INCLUDE=/home/foo/repo` (not
`/home/foo/repo/valgrind`)

This was just a small introduction, please see the
[docs](https://docs.rs/iai-callgrind/0.12.0/iai_callgrind/client_requests) for
more details!

### Flamegraphs

Flamegraphs are opt-in and can be created if you pass a `FlamegraphConfig` to
the `BinaryBenchmarkConfig`, `Run` or `LibraryBenchmarkConfig`. Callgrind
flamegraphs are meant as a complement to valgrind's visualization tools
`callgrind_annotate` and `kcachegrind`.

Callgrind flamegraphs show the inclusive costs for functions and a single
`EventKind` (default is `EventKind::Ir`), similar to `callgrind_annotate` but in
a nicer (and clickable) way. Especially, differential flamegraphs facilitate a
deeper understanding of code sections which cause a bottleneck or a performance
regressions etc.

The produced flamegraph `*.svg` files are located next to the respective callgrind
output file in the `target/iai` directory.

### Command-line arguments and environment variables

It's possible to pass arguments to iai-callgrind separated by `--` (`cargo bench
-- ARGS`). If you're running into the error `Unrecognized Option`, see the
[FAQ](#running-cargo-bench-results-in-an-unrecognized-option-error). For a
complete rundown of possible arguments, execute `cargo bench --bench <benchmark>
-- --help`. Almost all command-line arguments have a corresponding environment
variable. The environment variables which don't have a corresponding
command-line argument are:

- `IAI_CALLGRIND_COLOR`: Control the colored output of iai-callgrind. (Default
  is `auto`)
- `IAI_CALLGRIND_LOG`: Define the log level (Default is `WARN`)

#### Comparing with baselines

Usually, two consecutive benchmark runs let iai-callgrind compare these two
runs. It's sometimes desirable to compare the current benchmark run against a
static reference, instead. For example, if you're working longer on the
implementation of a feature, you may wish to compare against a baseline from
another branch or the commit from which you started off hacking on your new
feature to make sure you haven't introduced performance regressions.
`iai-callgrind` offers such custom baselines. If you are familiar with
[criterion.rs](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html#baselines),
the following command line arguments should also be very familiar to you:

- `--save-baseline=NAME`: Compare against the `NAME` baseline if present and
  then overwrite it. (env: `IAI_CALLGRIND_SAVE_BASELINE`)
- `--baseline=NAME`: Compare against the `NAME` baseline without overwriting it
  (env: `IAI_CALLGRIND_BASELINE`)
- `--load-baseline=NAME`: Load the `NAME` baseline as the `new` data set instead
  of creating a new one. This options needs also `--baseline=NAME` to be
  present. (env: `IAI_CALLGRIND_LOAD_BASELINE`)

If `NAME` is not present, `NAME` defaults to `default`.

For example to create a static reference from the main branch and compare it:

```shell
git checkout main
cargo bench --bench <benchmark> -- --save-baseline=main
git checkout feature
# ... HACK ... HACK
cargo bench --bench <benchmark> -- --baseline main
```

#### Customize the output directory

Per default, all benchmark output files are stored under the
`$PROJECT_ROOT/target/iai` directory tree. This home directory can be changed
with the `IAI_CALLGRIND_HOME` environment variable or the command-line argument
`--home`. The command-line argument overwrites the value of the environment
variable. For example to store all files under the `/tmp/iai-callgrind`
directory you can use `IAI_CALLGRIND_HOME=/tmp/iai-callgrind` or `cargo bench --
--home=/tmp/iai-callgrind`.

If you're running the benchmarks on different targets, it's necessary to
separate the output files of the benchmark runs per target or else you could end
up comparing the benchmarks with the wrong target leading to strange results.
You can achieve this with different baselines per target, but it's much less
painful to separate the output files by target with the `--separate-targets`
command-line argument or setting the environment variable
`IAI_CALLGRIND_SEPARATE_TARGETS=yes`). The output directory structure simply
changes from
`target/iai/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID` to
`target/iai/$TARGET_TRIPLE/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`.

For example the output directory of the following library benchmark assuming the
benchmark file name is `bench_file` in the package `my_package`:

```rust
use iai_callgrind::{main, library_benchmark_group, library_benchmark};
use my_lib::my_function;

#[library_benchmark]
#[bench::short(10)]
fn bench_function(value: u64) -> u64 {
    my_function(value)
}

library_benchmark_group!(
    name = bench_group;
    benchmarks = bench_function
);

main!(library_benchmark_groups = bench_group);
```

Without `--separate-targets`:

`target/iai/my_package/bench_file/bench_group/bench_function.short`

and with `--separate-targets` assuming you're running the benchmark on the
`x86_64-unknown-linux-gnu` target:

`target/iai/x86_64-unknown-linux-gnu/my_package/bench_file/bench_group/bench_function.short`

#### Machine-readable output

With `--output-format=default|json|pretty-json` (env:
`IAI_CALLGRIND_OUTPUT_FORMAT`) you can change the terminal output format to the
machine-readable json format. The json schema fully describing the json output
is stored in
[summary.v2.schema.json](./iai-callgrind-runner/schemas/summary.v2.schema.json).
Each line of json output (if not `pretty-json`) is a summary of a single
benchmark and you may want to combine all benchmarks in an array. You can do so
for example with `jq`

`cargo bench -- --output-format=json | jq -s`

which transforms `{...}\n{...}` into `[{...},{...}]`.

Instead of or in addition to changing the terminal output, it's possible to save
a summary file for each benchmark with `--save-summary=json|pretty-json` (env:
`IAI_CALLGRIND_SAVE_SUMMARY`). The `summary.json` files are stored next to the
usual benchmark output files in the `target/iai` directory.

#### Other output options

This section describes other command-line options and environment variables
which influence the terminal and logging output of iai-callgrind.

##### Show terminal output of benchmarks

Per default, all terminal output is captured and therefore not shown during a
benchmark run. To show any captured output, you can use
`IAI_CALLGRIND_LOG=info`. Another possibility is, to tell `iai-callgrind` to not
capture output with the `--nocapture` (env: `IAI_CALLGRIND_NOCAPTURE`) option.
This is currently restricted to the `callgrind` run to prevent showing the same
output multiple times. So, any terminal output of other tool runs (see also
[Valgrind Tools](#valgrind-tools)) is still captured.

The `--nocapture` flag takes the special values `stdout` and `stderr` in
addition to `true` and `false`. In the `--nocapture=stdout` case, terminal
output to `stdout` is not captured and shown during the benchmark run but output
to `stderr` is discarded. Likewise, `--nocapture=stderr` shows terminal output
to `stderr` but discards output to `stdout`.

For example a library benchmark `benches/my_benchmark.rs`

```rust
use iai_callgrind::{main, library_benchmark_group, library_benchmark};

fn my_teardown(value: u64) {
    eprintln!("Error output during teardown: {value}");
}

fn to_be_benchmarked(value: u64) -> u64 {
    println!("Output to stdout: {value}");
    value + 10
}

#[library_benchmark]
#[bench::some_id(args = (10), teardown = my_teardown]
fn my_bench(value: u64) -> u64 {
    to_be_benchmarked(value)
}

library_benchmark_group!(
    name = my_bench_group;
    benchmarks = my_bench
);

main!(library_benchmark_groups = my_bench_group);
```

If the above benchmark is run with `cargo bench --bench my_benchmark --
--nocapture`, the output of iai-callgrind will look like this (The values of
Instructions and so on don't matter here and are made up)

```text
my_benchmark::my_bench_group::my_bench some_id:10
Output to stdout: 10
Error output during teardown: 20
- end of stdout/stderr
  Instructions:              331082|N/A             (*********)
  L1 Hits:                   442452|N/A             (*********)
  L2 Hits:                      720|N/A             (*********)
  RAM Hits:                    3926|N/A             (*********)
  Total read+write:          447098|N/A             (*********)
  Estimated Cycles:          583462|N/A             (*********)
```

Note that independently of the value of the `--nocapture` option, all logging
output of a valgrind tool itself is stored in files in the output directory of
the benchmark. Since `iai-callgrind` needs the logging output of valgrind tools
stored in files, there is no option to disable the creation of these log files.
But, if anything goes sideways you might be glad to have the log files around.

##### Changing the color output

The terminal output is colored per default but follows the value for the
`IAI_CALLGRIND_COLOR` environment variable. If `IAI_CALLGRIND_COLOR` is not set,
`CARGO_TERM_COLOR` is also tried. Accepted values are: `always`, `never`, `auto`
(default). So, disabling colors can be achieved with setting
`IAI_CALLGRIND_COLOR` or `CARGO_TERM_COLOR=never`.

##### Changing the logging output

This library uses [env_logger](https://crates.io/crates/env_logger) and the
default logging level `WARN`. To set the logging level to something different,
set the environment variable `IAI_CALLGRIND_LOG` for example to
`IAI_CALLGRIND_LOG=DEBUG`. Accepted values are: `error`, `warn` (default),
`info`, `debug`, `trace`. The logging output is colored per default but follows
the settings of `IAI_CALLGRIND_COLOR` and `CARGO_TERM_COLOR` (In this order of
precedence). See also the [documentation](https://docs.rs/env_logger/latest) of
`env_logger`.

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
$ cargo bench --bench test_lib_bench_readme_example_fibonacci
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci short:10
  Instructions:                1733|N/A             (*********)
  L1 Hits:                     2359|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       2|N/A             (*********)
  Total read+write:            2361|N/A             (*********)
  Estimated Cycles:            2429|N/A             (*********)
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci long:30
  Instructions:            26214733|N/A             (*********)
  L1 Hits:                 35638617|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       4|N/A             (*********)
  Total read+write:        35638621|N/A             (*********)
  Estimated Cycles:        35638757|N/A             (*********)
```

For comparison, the output of the same benchmark but in the github CI is
producing the same results. Usually, there's almost no difference between a CI
run and a local run what makes benchmark runs and performance improvements of
the benchmarked code even more comparable across systems.

#### Cleaner output of Valgrind's annotation tools

The now obsolete calibration run needed with Iai has just fixed the summary output of Iai itself,
but the output of `cg_annotate` was still cluttered by the setup functions and metrics. The
`callgrind_annotate` output produced by Iai-Callgrind is far cleaner and centered on the actual
function under test.

#### Rework the metrics output

The statistics of the benchmarks are mostly not compatible with the original Iai anymore although
still related. They now also include some additional information:

```text
test_lib_bench_readme_example_fibonacci::bench_fibonacci_group::bench_fibonacci short:10
  Instructions:                1733|N/A             (*********)
  L1 Hits:                     2359|N/A             (*********)
  L2 Hits:                        0|N/A             (*********)
  RAM Hits:                       2|N/A             (*********)
  Total read+write:            2361|N/A             (*********)
  Estimated Cycles:            2429|N/A             (*********)
```

There is an additional line `Total read+write` which summarizes all event counters of the lines with
`Hits` above it and the `L1 Accesses` line changed to `L1 Hits`.

In detail:

`Total read+write = L1 Hits + L2 Hits + RAM Hits`.

The formula for the `Estimated Cycles` hasn't changed and uses Itamar Turner-Trauring's formula from
<https://pythonspeed.com/articles/consistent-benchmarking-in-ci/>:

`Estimated Cycles = L1 Hits + 5 × (L2 Hits) + 35 × (RAM Hits)`

For further details about how the caches are simulated and more, see the documentation of
[Callgrind](https://valgrind.org/docs/manual/cg-manual.html)

### What hasn't changed

Iai-Callgrind cannot completely remove the influences of setup changes. However, these effects
shouldn't be significant anymore.

### FAQ

#### I'm getting the error `Sentinel ... not found`

You've most likely disabled creating debug symbols in your cargo `bench`
profile. This can originate in an option you've added to the `release` profile
since the `bench` profile inherits the `release` profile. For example, if you've
added `strip = true` to your `release` profile which is perfectly fine, you need
to disable this option in your `bench` profile to be able to run `iai-callgrind`
benchmarks. See also the [Benchmarking](#benchmarking) section for a more
thorough example.

#### Running `cargo bench` results in an "Unrecognized Option" error

For `cargo bench -- --some-valid-arg` to work you can either specify the benchmark with
`--bench BENCHMARK`, for example `cargo bench --bench my_iai_benchmark --
--callgrind-args="--collect-bus=yes"` or add the following to your `Cargo.toml`:

```toml
[lib]
bench = false
```

and if you have binaries

```toml
[[bin]]
name = "my-binary"
path = "src/bin/my-binary.rs"
bench = false
```

Setting `bench = false` disables the creation of the implicit default `libtest`
harness which is added even if you haven't used `#[bench]` functions in your
library or binary. Naturally, the default harness doesn't know of the
`iai-callgrind` arguments and aborts execution printing the `Unrecognized
Option` error.

If you cannot or don't want to add `bench = false` to your `Cargo.toml`, you can
alternatively use environment variables. For every [command-line
argument](#command-line-arguments-and-environment-variables) exists a
corresponding environment variable.

### Contributing

A guideline about contributing to iai-callgrind can be found in the
[CONTRIBUTING.md](./CONTRIBUTING.md) file.

### See also

- The user guide of the original Iai: <https://bheisler.github.io/criterion.rs/book/iai/iai.html>
- A comparison of criterion-rs with Iai: <https://github.com/bheisler/iai#comparison-with-criterion-rs>

### Credits

Iai-Callgrind is forked from <https://github.com/bheisler/iai> and was originally written by Brook
Heisler (@bheisler).

Iai-Callgrind wouldn't be possible without [Valgrind](https://valgrind.org/).

### License

Iai-Callgrind is like Iai dual licensed under the Apache 2.0 license and the MIT
license at your option.

According to [Valgrind's documentation](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq):

> The Valgrind headers, unlike most of the rest of
the code, are under a BSD-style license so you may include them without worrying
about license incompatibility.

We have included the original license where we make use of the original header
files.

[`library documentation`]: https://docs.rs/iai-callgrind/0.12.0/iai_callgrind/
[docs]: https://docs.rs/iai-callgrind/0.12.0/iai_callgrind/
