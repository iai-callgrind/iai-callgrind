<!-- spell-checker:ignore serde -->
# Changelog

All notable changes to this project will be documented in this file.

This is the combined CHANGELOG for all packages: `iai-callgrind`, `iai-callgrind-runner` and
`iai-callgrind-macros`. `iai-callgrind` and `iai-callgrind-runner` use the same version which is the
version used here. `iai-callgrind-macros` uses a different version number but is not a standalone
package, so its changes are also listed here.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [0.7.0] - 2023-09-10

The old api to setup library benchmarks using only the `main!` macro is deprecated and was removed.
See the [README](./README.md) for a description of the new api.

Also, the api to setup binary benchmarks only with the `main!` macro is now deprecated and was
removed. Please use the builder api using the `binary_benchmark_groups!` and `Run`. The old binary
benchmark api lacked the rich possibilities of the builder api and maintaining two such different
apis adds a lot of unnecessary complexity.

Additionally, the scheme to setup binary benchmarks and specifying configuration options was
reworked and is now closer to the scheme how library benchmarks are set up. It's now possible to
specify a `BinaryBenchmarkConfig` at group level:

```rust
binary_benchmark_group!(
    name = some_name;
    config = BinaryBenchmarkConfig::default();
    benchmark = ...
)
```

`BinaryBenchmarkConfig` and `Run` received a lot of new methods to configure a binary benchmark run
at all levels from top-level `main!` via `binary_benchmark_group` down to `Run`.

### Added

* ([#5](https://github.com/Joining7943/iai-callgrind/issues/5)): Use a new attribute macro
(`#[library_benchmark]`) based api to setup library benchmarks. Also, bring the library benchmark
api closer to the binary benchmark api and use a `library_benchmark_group!` macro together with
`main!(library_benchmark_groups = ...)`
* `BinaryBenchmarkConfig` has new methods: `sandbox`, `fixtures`, `env`, `envs`, `pass_through_env`,
`pass_through_envs`, `env_clear`, `entry_point`, `current_dir`, `exit_with`
* `Run` has new methods: `pass_through_env`, `pass_through_envs`, `env_clear`, `entry_point`,
`current_dir`, `exit_with`, `raw_callgrind_args`
* It's now possible to specify a `BinaryBenchmarkConfig` at group level in the
`binary_benchmark_group!` macro with the argument `config = ...`
* `IAI_CALLGRIND_COLOR` environment variable which controls the color output of iai-callgrind. This
variable is now checked first before the usual `CARGO_TERM_COLOR`.

### Changed

* The output line `L1 Data Hits` changed to `L1 Hits` and in consequence now shows the event count
for instruction and data hits
* ([#7](https://github.com/Joining7943/iai-callgrind/issues/7)): Clear environment variables before
running library benchmarks. With that change comes the possibility to influence that behavior with
the `LibraryBenchmarkConfig::env_clear` method and set custom environment variables with
`LibraryBenchmarkConfig::envs`.
* ([15](https://github.com/Joining7943/iai-callgrind/issues/15)): Use `IAI_CALLGRIND` prefix for
iai-callgrind environment variables. `IAI_ALLOW_ASLR` -> `IAI_CALLGRIND_ALLOW_ASLR`, `RUST_LOG` ->
`IAI_CALLGRIND_LOG`.
* Callgrind invocations, if `IAI_CALLGRIND_LOG` level is `DEBUG` now runs Callgrind with `--verbose`
(This flag isn't documented in the official documentation of Callgrind)
* The signature of `Run::env` changed from `env(var: ...)` to `env(key: ... , value: ...)`
* The signature of `Run::envs` changed from `envs(vars: [String])` to
`envs(vars: [(Into<OsString>, Into<OsString>)])`
* The signatures of `Arg::new`, `Run::args`, `Run::with_args`, `Run::with_cmd_args` changed their
usage of `AsRef<[...]>` to  [`IntoIterator<Item = ...>`]

### Removed

* The old api from before [#5] using only the `main!` is now deprecated and the functionality
was removed. Using the old api produces a compile error. For migrating library benchmarks to the new
api see the [README](./README.md).
* `Run::options` and the`Options` struct were removed and all methods of this struct moved into `Run`
directly but are now also available in `BinaryBenchmarkConfig`.
* `BinaryBenchmarkGroup::fixtures` and `BinaryBenchmarkGroup::sandbox` were removed and they moved
to `BinaryBenchmarkConfig::fixtures` and `BinaryBenchmarkConfig::sandbox`

### Fixed

* ([#19](https://github.com/Joining7943/iai-callgrind/issues/19)): Library benchmark functions with
equal bodies produce event counts of zero.
* If the Callgrind arguments `--dump-instr=yes` and `dump-line=yes` were used together, the event
counters were summed up incorrectly.
* The Callgrind argument `--dump-every-bb` and similar arguments causing multiple file outputs
cannot be handled by `iai-callgrind` and therefore `--combine-dumps=yes` is now set per default.
This flag cannot be unset.
* `--compress-strings` is now ignored, because the parser needs the uncompressed strings or else
produces event counts of zero.
* Some debugging output was printed to stdout instead of stderr

### [0.6.2] - 2023-09-01

### Changed

* The dependency version requirements in all packages are loosened and more openly formulated.
Especially, the upper bounds were updated to include the latest versions. However, the `Cargo.lock`
file locks the dependencies to versions which are compatible with the current MSRV `1.60.0`.

### Fixed

* The `iai-callgrind` package was unnecessarily using all the dependencies of the
`iai-callgrind-runner` although only dependent on the `api` feature of the runner. Also, the direct
`serde` dependency was removed because `serde` is already part of the `api` feature of the runner.
* Changed the license from `Apache-2.0 AND MIT` to `Apache-2.0 OR MIT` in Cargo.toml
files of all packages

### [0.6.1] - 2023-08-25

### Fixed

* ([#4](https://github.com/Joining7943/iai-callgrind/issues/4)): The destination
directory of iai callgrind output files changes from `/workspace/$CARGO_PKG_NAME/target/iai` to
`/workspace/target/iai/$CARGO_PKG_NAME` and respects the `CARGO_TARGET_DIR` environment variable

### [0.6.0] - 2023-08-20

### Added

* ([#3](https://github.com/Joining7943/iai-callgrind/issues/3)): builder api for binary benchmarks

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

* ([#2](https://github.com/Joining7943/iai-callgrind/issues/2)): Benchmarking binaries of a crate.
Added a full description of this benchmarking scheme in the README
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
