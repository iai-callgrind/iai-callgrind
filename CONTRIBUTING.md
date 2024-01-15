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

All patches have to be sent on Github as [pull
requests](https://github.com/iai-callgrind/iai-callgrind/pulls). Before starting
a pull request, it is best to open an issue first so no efforts are wasted.

If you are looking for a place to start contributing to iai-callgrind, take a
look at the [help
wanted](https://github.com/iai-callgrind/iai-callgrind/labels/help%20wanted) or
[good first
issue](https://github.com/iai-callgrind/iai-callgrind/labels/good%20first%20issue)
issues.

The minimum supported version (MSRV) of iai-callgrind is Rust `1.66.0` and all
patches are expected to work with the minimum supported version.

All notable changes need to be added to the
[CHANGELOG](https://github.com/iai-callgrind/iai-callgrind/blob/4f29964c153a2dd20283fb1502db3de630148629/CHANGELOG.md).

## How to get started

Clone this repo

```shell
git clone https://github.com/iai-callgrind/iai-callgrind.git
```

and then change the MSRV locally

```shell
cd iai-callgrind
rustup override set 1.66.0
```

What is left is to setup your favorite editor to use nightly rustfmt and clippy
from the rust `stable` toolchain in order to pass the formatting and linting
checks in the `ci`.

## Testing

iai-callgrind lacks tests and contributions expanding the test suite are also
very welcome. Patches have to include tests to verify (at a minimum) that the
whole pipeline runs through without errors.

The benches in the `benchmark-tests` package run the whole pipeline which is
good for verifying that there are no panics or errors. If you've made changes in
the `iai-callgrind-runner` package then you can point the `IAI_CALLGRIND_RUNNER`
environment variable to your modified version of the `iai-callgrind-runner`
binary:

```rust
cargo build -p iai-callgrind-runner --release
IAI_CALLGRIND_RUNNER=$(readlink -e target/release/iai-callgrind-runner) cargo bench -p benchmark-tests
```

The concrete results of the benchmarks are not checked with the
`benchmark-tests`. Use unit tests or integration tests in the
`iai-callgrind-runner` package to test for concrete results.

## Contact

If there are any outstanding questions about contributing to iai-callgrind, they
can be asked on the [iai-callgrind issue
tracker](https://github.com/iai-callgrind/iai-callgrind/issues).
