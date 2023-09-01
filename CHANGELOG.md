<!-- spell-checker:ignore serde -->
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [0.6.2] - 2023-09-01

### Changed

* The dependency version requirements in all packages are loosened and more openly formulated.
Especially, the upper bounds were updated to include the latest versions.

### Fixed

* The `iai-callgrind` package was unnecessarily using all the dependencies of the
`iai-callgrind-runner` although only dependent on the `api` feature of the runner. Also, the direct
`serde` dependency was removed because `serde` is already part of the `api` feature of the runner.
* Changed the license from `Apache-2.0 AND MIT` to `Apache-2.0 OR MIT` in Cargo.toml
files of all packages

### [0.6.1] - 2023-08-25

### Fixed

* (#4) The destination directory of iai callgrind output files changes from
`/workspace/$CARGO_PKG_NAME/target/iai` to `/workspace/target/iai/$CARGO_PKG_NAME` and respects the
`CARGO_TARGET_DIR` environment variable

### [0.6.0] - 2023-08-20

### Added

* (#3) builder api for binary benchmarks

### Changed

* BREAKING: an id for args in the macro api is now mandatory
* binary benchmarks: The filename of callgrind output for benchmarked `setup`, `teardown`, `before` and `after`
functions changed to `callgrind.$id.$function.out`.  
* binary benchmarks: The filename of callgrind output for benchmarked binaries does not include the arguments for the
binary anymore.

### Fixed

* The filename for callgrind output files is now truncated to a maximum of 255 bytes
* library benchmarks: Fix event counting to include costs of inlined functions

### [0.5.0] - 2023-08-07

### Added

* (#2) Benchmarking binaries of a crate. Add full description of this benchmarking scheme in the
README
* IAI_CALLGRIND_RUNNER environment variable which may specify the path to the iai-callgrind-runner
binary

### Changed

* The error output changed and double information was removed when running the
`iai-callgrind-runner` fails
* The architecture detection changed from using `uname -m` to use rust's `std::env::consts::ARCH`

### Removed

* The cfg_if dependency was removed

### Fixed

* If running with ASLR disabled, proccontrol on freebsd was missing to run the valgrind binary

### [0.4.0] - 2023-07-19

BREAKING: Counting of events changed and therefore event counters are incompatible with versions
before `0.4.0`. Usually, event counters are now lower and more precise than before.

### Changed

* Instead of counting all events within the benchmarking function, only events of function calls
(cfn entries) within the benchmarking functions are attributed to the final event counts.
* MSRV changed from v1.56.0 -> v1.60.0
* Bump log dependency 0.4.17 -> 0.4.19

### Fixed

* Counting of events was sometimes summarizing the events of the `main` function instead of the
benchmarking function

### [0.3.1] - 2023-03-13

### Added

* Add output of Callgrind at `RUST_LOG=info` level but also more debug and trace output.

### Fixed

* The version mismatch check should cause an error when the library version is < 0.3.0

### [0.3.0] - 2023-03-13

This version is incompatible to previous versions due to changes in the `main!` macro which is
passing additional arguments to the runner. However, benchmarks written with a version before
`v0.3.0` don't need any changes but can take advantage of some new features.

### Changed

* The `toggle-collect` callgrind argument now accumulates multiple occurrences instead of replacing
them. The default `toggle-collect` for the benchmark function cannot be replaced anymore.
* A version mismatch of the `iai-callgrind` library and the `iai-callgrind-runner` is now an error.
* Fix, update and extend the README. Add more real-world examples.

### Added

* The `main!` macro has two forms now, with the first having the ability to pass arguments to
callgrind.
* More examples in the benches folder
* Use the `RUST_LOG` environment variable to control the verbosity level of the runner.
* Add colored output. The `CARGO_TERM_COLOR` variable can be used to disable colors.

### Fixed

* A cargo filter argument which is a positional argument resulted in the the runner to crash.

### [0.2.0] - 2023-03-10

This version is mostly compatible with `v0.1.0` but needs some additional setup. See
[Installation](README.md#installation) in the README. Benchmarks created with `v0.1.0` should not
need any changes but can maybe improved with the additional features from this version.

### Changed

* The repository layout changed and this package is now separated in a library
(iai-callgrind) with the main macro and the black_box and the binary package (iai-callgrind-runner)
with the runner needed to run the benchmarks
* It's now possible to pass additional arguments to callgrind
* The output of the collected event counters and metrics has changed
* Other improvements to stabilize the metrics across different systems

### [0.1.0] - 2023-03-08

### Added

* Initial migration from Iai
