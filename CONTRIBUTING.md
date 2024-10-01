<!-- spell-checker: ignore readlink -->

# Contributing to iai-callgrind

Thank you for your interest in contributing to iai-callgrind!

## Feature Requests and Bug reports

Feature requests and bug reports should be reported in the [Issue
Tracker](https://github.com/iai-callgrind/iai-callgrind/issues). Please have a
look at existing issues with the
[enhancement](https://github.com/iai-callgrind/iai-callgrind/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)
or
[bug](https://github.com/iai-callgrind/iai-callgrind/issues?q=is%3Aissue+is%3Aopen+label%3Abug)
labels.

## Patches / Pull Requests

All patches have to be sent on GitHub as [pull
requests](https://github.com/iai-callgrind/iai-callgrind/pulls). Before starting
a pull request, it is best to open an issue first so no efforts are wasted.

If you are looking for a place to start contributing to iai-callgrind, take a
look at the [help
wanted](https://github.com/iai-callgrind/iai-callgrind/labels/help%20wanted) or
[good first
issue](https://github.com/iai-callgrind/iai-callgrind/labels/good%20first%20issue)
issues.

The minimum supported version (MSRV) of iai-callgrind is Rust `1.75.0` and all
patches are expected to work with the minimum supported version.

All notable changes need to be added to the
[CHANGELOG](https://github.com/iai-callgrind/iai-callgrind/blob/4f29964c153a2dd20283fb1502db3de630148629/CHANGELOG.md).

## How to get started

Clone this repo

```shell
git clone https://github.com/iai-callgrind/iai-callgrind.git
cd iai-callgrind
```

Working on this project is a piece of cake with
[just](https://github.com/casey/just) and if you have the `just` shell
completions installed. Before running any install commands with `just`, it is
recommended to first inspect it with `--dry-run`. Install the basics needed to
start working on this project with:

```shell
just install-workspace
```

This command will install git hooks, the necessary components for the `stable`,
`nightly` toolchain and the current MSRV toolchain, run some checks for tools
which need to be installed, ...

To get an overview over all possible `just` rules run `just -l` or directly
inspect the `Justfile` in the root of this project.

If your IDE can handle it, it's usually best to work with the MSRV locally

```shell
rustup override set 1.75.0
```

What is left is to set up your favorite editor to use nightly rustfmt and clippy
from the rust `stable` toolchain in order to pass the formatting and linting
checks in the `ci`.

## Testing

Patches have to include tests to verify (at a minimum) that the whole pipeline
runs through without errors.

The benches in the `benchmark-tests` package are system tests and run the whole
pipeline. We use a wrapper around `cargo --bench` (`benchmark-tests/src/bench`)
to run the `benchmark-tests`. In order to run a single benchmark-tests use `just
full-bench-test $BENCHMARK_NAME` or all with `just full-bench-test-all`.

The user interface is tested in `iai-callgrind/tests/ui`.

If you've made changes in the `iai-callgrind-runner` package then you can point
the `IAI_CALLGRIND_RUNNER` environment variable to your modified version of the
`iai-callgrind-runner` binary:

```shell
cargo build -p iai-callgrind-runner --release
IAI_CALLGRIND_RUNNER=$(readlink -e target/release/iai-callgrind-runner) cargo bench -p benchmark-tests
```

or with `just` in a single command:

```shell
just bench-test-all
```

or a specific bench of the benchmark-test package

```shell
just bench-test test_lib_bench_tools
```

The concrete results of the benchmarks are not checked when running the
benchmark tests that way. You can use

```shell
just full-bench-test test_lib_bench_tools
```

which also checks that any output files that are expected to be created by the
benchmark run are actually there. Depending on the test configuration benchmarks
are sometimes run multiple times.

## Contact

If there are any outstanding questions about contributing to iai-callgrind, they
can be asked on the [iai-callgrind issue
tracker](https://github.com/iai-callgrind/iai-callgrind/issues).
