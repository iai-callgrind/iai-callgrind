<!-- spell-checker: ignore fixt -->

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
        <img src="https://img.shields.io/badge/MSRV-1.60.0-brightgreen" alt="MSRV"/>
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
    - [Flamegraphs](#flamegraphs)
    - [Iai-callgrind Environment variables](#iai_callgrind-environment-variables)
    - [Iai-callgrind command line arguments](#command-line-passing-arguments-to-callgrind)
    - [Features and differences to Iai](#features-and-differences-to-iai)
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
- __Stable-compatible__: Benchmark your code without installing nightly Rust

### Installation

In order to use Iai-Callgrind, you must have [Valgrind](https://www.valgrind.org) installed. This
means that Iai-Callgrind cannot be used on platforms that are not supported by Valgrind.

To start with Iai-Callgrind, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
iai-callgrind = "0.8.0"
```

To be able to run the benchmarks you'll also need the `iai-callgrind-runner` binary installed
somewhere in your `$PATH`, for example with

```shell
cargo install --version 0.8.0 iai-callgrind-runner
```

There's also the possibility to install the binary somewhere else and point the
`IAI_CALLGRIND_RUNNER` environment variable to the absolute path of the `iai-callgrind-runner`
binary like so:

```shell
cargo install --version 0.8.0 --root /tmp iai-callgrind-runner
IAI_CALLGRIND_RUNNER=/tmp/bin/iai-callgrind-runner cargo bench --bench my-bench
```

When updating the `iai-callgrind` library, you'll also need to update `iai-callgrind-runner` and
vice-versa or else the benchmark runner will exit with an error.

### Benchmarking

`iai-callgrind` can be used to benchmark libraries or binaries. Library benchmarks benchmark
functions and methods of a crate and binary benchmarks benchmark the executables of a crate. The
different benchmark types cannot be intermixed in the same benchmark file but having different
benchmark files for library and binary benchmarks is no problem. More on that in the following
sections.

For a quickstart and examples of benchmarking libraries see the [Library Benchmark
Section](#library-benchmarks) and for executables see the [Binary Benchmark
Section](#binary-benchmarks). Read the [docs]!

It's highly advisable to run the benchmarks with debugging symbols switched on.
For example in your `~/.cargo/config`:

```toml
[profile.bench]
debug = true
```

Now, all benchmarks you run with `cargo bench` include the debug info. (See also
[Cargo Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and
[Cargo Config](https://doc.rust-lang.org/cargo/reference/config.html))

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
use iai_callgrind::{black_box, main, library_benchmark_group, library_benchmark};

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

You'll find the callgrind output files of each run of the benchmark `my_binary_benchmark` of the
group `my_exe_group` in `target/iai/$CARGO_PKG_NAME/my_binary_benchmark/my_exe_group`.

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
and you have to opt-in with a `RegressionConfig` or [Environment
variables](#iai_callgrind-environment-variables).

A performance regression check consists of an `EventKind` and a percentage over
which a regression is assumed. If the percentage is negative, then a regression
is assumed to be below this limit. The default `EventKind` is
`EventKind::EstimatedCycles` with a value of `+10%`.For example, in a [Library
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

### Flamegraphs

Flamegraphs are opt-in and can be created if you pass a `FlamegraphConfig` to
the `BinaryBenchmarkConfig`, `Run` or `LibraryBenchmarkConfig`. Callgrind
flamegraphs are meant as a complement to valgrind's visualization tools
`callgrind_annotate` and `kcachegrind`.

Callgrind flamegraphs show the inclusive costs for functions and a specific
event type, similar to `callgrind_annotate` but in a nicer (and clickable) way.
Especially, differential flamegraphs facilitate a deeper understanding of code
sections which cause a bottleneck or a performance regressions etc.

The produced flamegraph svg files are located next to the respective callgrind
output file in the `target/iai` directory.

### IAI_CALLGRIND Environment variables

This is an overview of environment variables iai-callgrind understands:

- `IAI_CALLGRIND_COLOR`: Control the colored output of iai-callgrind
- `IAI_CALLGRIND_LOG`: Define the log level
- `IAI_CALLGRIND_REGRESSION`: Define limits for event kinds to detect performance
  regressions
- `IAI_CALLGRIND_REGRESSION_FAIL_FAST`: If `yes`, fail the benchmarks on the first
  performance regression encountered. The default is `no`.

#### IAI_CALLGRIND_COLOR

The metrics output is colored per default but follows the value for the `IAI_CALLGRIND_COLOR`
environment variable. If `IAI_CALLGRIND_COLOR` is not set, `CARGO_TERM_COLOR` is also tried.
Accepted values are: `always`, `never`, `auto` (default). So, disabling colors can be achieved with
setting `IAI_CALLGRIND_COLOR` or `CARGO_TERM_COLOR=never`.

#### IAI_CALLGRIND_LOG

This library uses [env_logger](https://crates.io/crates/env_logger) and the default logging level
`WARN`. To set the logging level to something different, set the environment variable
`IAI_CALLGRIND_LOG` for example to `IAI_CALLGRIND_LOG=DEBUG`. Accepted values are: `error`, `warn`
(default), `info`, `debug`, `trace`. The logging output is colored per default but follows the
settings of `IAI_CALLGRIND_COLOR` and `CARGO_TERM_COLOR` (In this order). See also the
[documentation](https://docs.rs/env_logger/latest) of `env_logger`.

#### IAI_CALLGRIND_REGRESSION

This environment variables takes a `,` separated list of `EVENT_KIND=PERCENTAGE`
(key=value) pairs. For example `IAI_CALLGRIND_REGRESSION='Ir=5,
EstimatedCycles=10'`. See also the section about [Performance
Regressions](#performance-regressions).

#### IAI_CALLGRIND_REGRESSION_FAIL_FAST

This environment variables takes `yes` or `no` as value for example
`IAI_CALLGRIND_REGRESSION_FAIL_FAST=yes`. This environment variable will be
ignored if no `IAI_CALLGRIND_REGRESSION` variable is defined. See also the
section about [Performance Regressions](#performance-regressions).

### Command-line: Passing arguments to Callgrind

It's now possible to pass additional arguments to callgrind separated by `--` (`cargo bench --
CALLGRIND_ARGS`) or overwrite the defaults, which are:

- `--I1=32768,8,64`
- `--D1=32768,8,64`
- `--LL=8388608,16,64`
- `--toggle-collect` (additive)
- `--collect-atstart=no`
- `--compress-pos=no`

Note that `toggle-collect` won't be overwritten by any additional `toggle-collect` argument but
instead will be passed to Callgrind in addition to the default value in the case of [library
benchmarks](#library-benchmarks). [Binary benchmarks](#binary-benchmarks) don't have a default
toggle.

Some callgrind arguments don't play well with `iai-callgrind`'s defaults and are therefore ignored:

- `--separate-threads`
- `--callgrind-out-file`
- `--cache-sim`
- `--compress-strings`
- `--combine-dumps`

See also [Callgrind Command-line Options](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options).

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

For comparison, the output of the same benchmark but in the github CI, producing
the exact same results:

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

There's no difference (or only very small differences) what makes benchmark runs
and performance improvements of the benchmarked code even more comparable across
systems.

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

#### Incomplete list of other minor improvements

- The output files of Callgrind are now located in a subdirectory under `target/iai` to avoid
overwriting them in case of multiple benchmark files.

### What hasn't changed

Iai-Callgrind cannot completely remove the influences of setup changes. However, these effects
shouldn't be significant anymore.

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

Iai-Callgrind is like Iai dual licensed under the Apache 2.0 license and the MIT license at your
option.

[`library documentation`]: https://docs.rs/iai-callgrind/0.8.0/iai_callgrind/
[docs]: https://docs.rs/iai-callgrind/0.8.0/iai_callgrind/
