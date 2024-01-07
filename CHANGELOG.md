<!-- spell-checker:ignore serde -->
<!--
Added for new features.
Changed for changes in existing functionality.
Deprecated for soon-to-be removed features.
Removed for now removed features.
Fixed for any bug fixes.
Security in case of vulnerabilities.
-->

# Changelog

All notable changes to this project will be documented in this file.

This is the combined CHANGELOG for all packages: `iai-callgrind`, `iai-callgrind-runner` and
`iai-callgrind-macros`. `iai-callgrind` and `iai-callgrind-runner` use the same version which is the
version used here. `iai-callgrind-macros` uses a different version number but is not a standalone
package, so its changes are also listed here.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

* ([#42](https://github.com/iai-callgrind/iai-callgrind/issues/42)): Support
  valgrind client requests. The client requests are available in the
  `iai-callgrind` package and can be activated via feature flags
  (`client_requests` and `client_requests_defs`).
* ([#38](https://github.com/iai-callgrind/iai-callgrind/issues/38)): Add support
  for specifying multiple library benchmarks in one go with the `#[benches]`
  attribute. This attribute also accepts a `setup` argument which takes a path
  to a function, so the `args` are passed as parameter to the `setup` function
  instead of the benchmarking function.

### Changed

* ([#48](https://github.com/iai-callgrind/iai-callgrind/pull/48)): Update MSRV
  from `1.60.0` to `1.66.0`. Make use of new language features.
* ([#48](https://github.com/iai-callgrind/iai-callgrind/pull/48)): Update
  dependencies. Use latest possible versions (with our MSRV) of `which`,
  `cargo_metadata`, `indexmap`, `clap` and others.

### Deprecated

* ([#48](https://github.com/iai-callgrind/iai-callgrind/pull/48)): Change our
  implementation of `black_box` to wrap `std::hint::black_box` which is stable
  since `1.66.0`. The usage of `iai_callgrind::black_box` is deprecated and
  `std::hint::black_box` should be used directly.

### Fixed

* ([#48](https://github.com/iai-callgrind/iai-callgrind/pull/48)): The
  `lazy_static` dependency of `iai-callgrind-runner` is now optional and not
  unnecessarily installed with the `iai-callgrind` package.

### [0.9.0] - 2023-12-09

### Added

* ([#31](https://github.com/iai-callgrind/iai-callgrind/issues/31)): Machine
  readable output. This feature adds an environment variable
  `IAI_CALLGRIND_SAVE_SUMMARY` and command line argument `--save-summary` to
  create a `summary.json` next to the usual output files of a benchmark which
  contains all the terminal output data and more in a machine readable output
  format. The json schema for the json summary file is stored in
  `iai-callgrind-runner/schemas/*.json`. In addition to `--save-summary` and
  saving the summary to a file it's possible with
  `--output-format=default|json|pretty-json` to specify
  the output format for the terminal output.
* Add command line arguments `--allow-aslr`, `--regression` and
  `--regression-fail-fast` which have higher precedence than their environment
  variable counterparts `IAI_CALLGRIND_ALLOW_ASLR`, `IAI_CALLGRIND_REGRESSION`
  and `IAI_CALLGRIND_REGRESSION_FAIL_FAST`
* ([#29](https://github.com/iai-callgrind/iai-callgrind/issues/29)): Add support
  to compare against baselines instead of the usual `*.old` output files. This
  adds command-line arguments `--save-baseline=BASELINE`,
  `--load-baseline=BASELINE` and `--baseline=BASELINE` and their environment
  variable counterparts `IAI_CALLGRIND_SAVE_BASELINE`,
  `IAI_CALLGRIND_LOAD_BASELINE` and `IAI_CALLGRIND_BASELINE`.
* ([#30](https://github.com/iai-callgrind/iai-callgrind/issues/30)): Add
  environment variable `IAI_CALLGRIND_CALLGRIND_ARGS` as complement to
  `--callgrind-args`

### Changed

* Like discussed in #31, the parsing of command line arguments for iai-callgrind
  in `cargo bench ... -- ARGS` had to change. Instead of interpreting all `ARGS`
  as Callgrind arguments, Callgrind arguments can now be passed with the
  `--callgrind-args=...` option, so other iai-callgrind arguments are now
  possible, for example the `--save-summary=...` option in #31 or even `--help`
  and `--version`.
* The names of output files and directories of binary benchmarks changed the
  order from `ID.BINARY` to `BINARY.ID` to match the file naming scheme
  `FUNCTION.ID` of library benchmarks.
* ([#35](https://github.com/iai-callgrind/iai-callgrind/issues/35)): The
  terminal output of other valgrind tool runs (like Memcheck, DRD, ...) is now
  more informative and also shows the content of the log file, if any. If not
  specified otherwise, Memcheck, DRD and Helgrind now run with
  `--error-exitcode=201`. If any errors are detected by these tools, setting
  this option to an exit code different from `0` causes the benchmark run to
  fail immediately and show the whole logging output.
* The output file names of flamegraphs had to change due to #29.
* All output not being part of the summary terminal output now goes to stderr.
  This change affects the logging output at `info` level and the regression
  check output.

### Fixed

* The `iai-callgrind-runner` dependencies `regex` and `glob` were removed from
  the `iai-callgrind` dependencies.
* The `stderr` output from a valgrind run wasn't shown in case of an error
  during the benchmark run because of the change to use `--log-file` to store
  valgrind output in log files. However, not all valgrind output goes into the
  log file in case of an error, so it is still necessary to print the `stderr`
  output after the log file content to see all error output of valgrind.
* Update the yanked wasm-bindgen `0.2.88` to `0.2.89`

### [0.8.0] - 2023-11-10

### Added

* ([#6](https://github.com/iai-callgrind/iai-callgrind/issues/6)): Show and fail
  benchmarks on performance regressions. Configuration of regression checks can
  be done with `RegressionConfig` or with the new environment variables
  `IAI_CALLGRIND_REGRESSION` and `IAI_CALLGRIND_REGRESSION_FAIL_FAST`
* ([#26](https://github.com/iai-callgrind/iai-callgrind/issues/26)): Show event
  kinds which are not associated with callgrind's cache simulation if available.
  For example, running callgrind with flags like `--collect-systime`
  (`SysCount`, `SysTime`, `SysCpuTime`), ...
* ([#18](https://github.com/iai-callgrind/iai-callgrind/issues/18)): Add support
  for DHAT, Massif, BBV, Memcheck, Helgrind, DRD. It's now possible to run each
  of these tools for each benchmark (in addition to callgrind). The output files
  of the profiling tools DHAT, Massif and BBV can be found next to the usual
  callgrind output files.

### Changed

* The output format was reworked and now shows the old event counts next to
  the new event counts instead of just the new event counts.
* The output format now shows the factor in addition to the percentage
  difference when comparing the new benchmark run with the old benchmark run.
  The factor can be more intuitive when trying to estimate performance
  improvements.
* The output format also received some small improvements in case a cost is not
  recorded either in the new benchmark run or in the old benchmark run.
* The percentage difference is now a digit shorter to equalize the widths of the
  different other string outputs within the parentheses.
* Due to the additional possible output files from tools like DHAT, Massif, etc.
  (but also flamegraphs), the output of benchmark runs is now nested one level
  deeper into a directory for each benchmark id instead of putting all output
  files into the group directory.
* Passing short options (like `-v`) to
  `LibraryBenchmarkConfig::raw_callgrind_args`,
  `BinaryBenchmarkConfig::raw_callgrind_args`, `Run::raw_callgrind_args`
  `Tool::args` is now possible
* The output of iai-callgrind when running multiple tool was adjusted
* `--log-file` for callgrind runs is now ignored because the log files are now
  created and placed next to the usual output files of iai-callgrind
* `-q`, `--quiet` arguments are now ignored because they are known to cause
  problems when parsing log file output for example for DHAT.

### Fixed

* Fix examples README to show the correct summary costs of events
* Fix error handling if valgrind terminates abnormally or with a signal instead
  of an exit code
* Fixed missing flamegraph creation when running setup, after, before and
  teardown functions in binary benchmarks if `bench` is set to `true`.
* Running callgrind with `--compress-pos=yes` is currently incompatible with
  iai-callgrind's parsing of callgrind output files. If this option is given, it
  will be ignored.
* Running iai-callgrind with valgrind's options `--help`, `-h`, `--help-debug`,
  `--help-dyn-options`, `--version` may cause problems and these arguments are now
  ignored.

### [0.7.3] - 2023-10-24

### Changed

* Update repository to use github organization `iai-callgrind/iai-callgrind`
* Lower the locked inferno dependency to `0.11.12` to workaround yanked `ahash`
  version `0.8.3`

### [0.7.2] - 2023-10-18

### Added

* ([#23](https://github.com/iai-callgrind/iai-callgrind/issues/23)): Create
  regular and differential flamegraphs from callgrind output.

### Fixed

* ([#22](https://github.com/iai-callgrind/iai-callgrind/pull/22)): Clearify how
  to update iai-callgrind-runner
* Some small fixes of parsing callgrind output files in the event that no
  records are present.

### [0.7.1] - 2023-09-27

### Fixed

* ([#20](https://github.com/iai-callgrind/iai-callgrind/issues/20)): Clearing the
  environment variables with `env_clear` may break finding valgrind.

### [0.7.0] - 2023-09-21

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

* ([#5](https://github.com/iai-callgrind/iai-callgrind/issues/5)): Use a new attribute macro
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
* ([#7](https://github.com/iai-callgrind/iai-callgrind/issues/7)): Clear environment variables before
running library benchmarks. With that change comes the possibility to influence that behavior with
the `LibraryBenchmarkConfig::env_clear` method and set custom environment variables with
`LibraryBenchmarkConfig::envs`.
* ([#15](https://github.com/iai-callgrind/iai-callgrind/issues/15)): Use `IAI_CALLGRIND` prefix for
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

* ([#19](https://github.com/iai-callgrind/iai-callgrind/issues/19)): Library benchmark functions with
equal bodies produce event counts of zero.
* If the Callgrind arguments `--dump-instr=yes` and `dump-line=yes` were used together, the event
counters were summed up incorrectly.
* The Callgrind argument `--dump-every-bb` and similar arguments causing multiple file outputs
cannot be handled by `iai-callgrind` and therefore `--combine-dumps=yes` is now set per default.
This flag cannot be unset.
* `--compress-strings` is now ignored, because the parser needs the uncompressed strings or else
produces event counts of zero.
* Some debugging output was printed to stdout instead of stderr
* Adjust parsing of yes/no values from `LibraryBenchmarkConfig` and `BinaryBenchmarkConfig` raw
callgrind arguments to callgrind's parsing of command-line arguments. Now, only exact matches of
`yes` and `no` are considered to be valid command-line arguments.

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

* ([#4](https://github.com/iai-callgrind/iai-callgrind/issues/4)): The destination
directory of iai callgrind output files changes from `/workspace/$CARGO_PKG_NAME/target/iai` to
`/workspace/target/iai/$CARGO_PKG_NAME` and respects the `CARGO_TARGET_DIR` environment variable

### [0.6.0] - 2023-08-20

### Added

* ([#3](https://github.com/iai-callgrind/iai-callgrind/issues/3)): builder api for binary benchmarks

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

* ([#2](https://github.com/iai-callgrind/iai-callgrind/issues/2)): Benchmarking binaries of a crate.
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
