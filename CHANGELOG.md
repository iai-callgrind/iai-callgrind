<!-- spell-checker:ignore serde dewert binstall jembishop kehl DaniPopes bytemuck hargut -->
<!-- spell-checker:ignore ryanpeach hashbrown tgross35 gaetschwartz cfgs -->
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

This release includes breaking changes especially for DHAT explained in more
detail below.

__Breaking__:

* DHAT is now considered to be fully integrated into Iai-Callgrind and the
  default entry point in library benchmarks for DHAT is set to the benchmark
  function similar to callgrind. This changes the metrics of the DHAT output.

In contrast to callgrind, the DHAT default entry point *includes* any
(de-)allocations in `setup` and/or `teardown` code in order to be able to detect
the DHAT metrics of the benchmark itself. The inclusion of `setup`/`teardown`
metrics is a limitation of DHAT and what is possible to reliably extract from
the output files. Nevertheless, this change allows the metrics to be centered on
the benchmark function excluding the heap operations of Iai-Callgrind needed to
setup the benchmark (allocations fluctuating between 2000 - 2500 bytes). More
importantly, the DHAT metrics are stabilized. This clears the way for setting
regression limits in the same fashion as it is possible for callgrind.
Additionally, it is possible to specify additional frames/functions to be
included/excluded from the DHAT metrics similar to what `--toggle-collect` does
for callgrind.

__Breaking__:

* Note that in DHAT library benchmarks of multi-threaded/multi-process
  functions, the threads/subprocesses are not included in the metrics anymore.
  This is exactly equivalent to the situation in callgrind benchmarks of
  multi-threaded/multi-process functions and can be solved in the same way with
  additional `Dhat::frames`.

Also coming with this release: The distinction between soft and hard limits.
Soft limits are now what was before just "the limits" and describe limits for the
percentage difference between the "new" and "old" run. Hard limits cap the
metrics of the "new" run in absolute numbers.

__Breaking__:

* This changes the way command-line arguments like `--callgrind-limits`,
  `--dhat-limits`, ... are parsed and to be able to disambiguate between soft
  and hard limits, soft limits have to be suffixed with a `%`.

Soft limit before: `--callgrind-limits='ir=0.0'`<br>
After:  `--callgrind-limits='ir=0.0%'` or  `--callgrind-limits='ir=0%'`

Hard limits are bare numbers: `--callgrind-limits='ir=10000'`.

Also new: It's possible to specify soft or hard limits for whole groups like
`--callgrind-limits='@all=5%'`.

### Added

* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): Added the
  method `Dhat::entry_point` to be able to change the default entry point
  similar to `Callgrind::entry_point` and `Dhat::frames` to be able to specify
  additional functions in a similar way to `--toggle-collect` of callgrind.
* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): Possibility
  to specify dhat regression limits with the `Dhat` struct and with the
  command-line argument `--dhat-limits` or environment variable
  `IAI_CALLGRIND_DHAT_LIMITS`.
* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): New dhat
  metrics `DhatMetric::TotalUnits` and `DhatMetric::TotalEvents` for dhat
  `ad-hoc` mode. They are part of the default output format of dhat.
* ([#407](https://github.com/iai-callgrind/iai-callgrind/pull/407))!: Add
  possibility to specify hard limits in addition to soft limits. This breaks the
  parsing of the `--callgrind-limits`, ... arguments. To disambiguate between
  hard and soft limits the soft limits have to be suffixed with a `%`. New
  methods `Callgrind::soft_limits`, `Callgrind::hard_limits`,
  `Cachegrind::soft_limits`, `Cachegrind::hard_limits`, `Dhat::soft_limits`,
  `Dhat::hard_limits`.
* ([#407](https://github.com/iai-callgrind/iai-callgrind/pull/407)): Add soft or
  hard limits for whole groups like in `--callgrind-limits='@all=5%'`. The
  `--help` message for `--callgrind-limits`, ... shows all possible groups.

### Changed

* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406))!: In library
  benchmarks, the default entry point for dhat is now the benchmark function
  `EntryPoint::Default`. As opposed to callgrind benchmarks, this includes the
  (de-)allocations/metrics of a `setup` and/or `teardown` function. For binary
  benchmarks nothing has changed and the default entry point is set to none with
  `EntryPoint::None`.
* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): Improved
  error message: Changed `Invalid format of key/value pair: '{split}'` to
  `Invalid format of key=value pair: '{split}'`.
* ([#407](https://github.com/iai-callgrind/iai-callgrind/pull/407)): Make the
  expanded benchmark function module public with `pub mod` instead of just
  `mod`. This allows putting benchmark functions into modules and adding them
  into groups outside of this module.
* ([#407](https://github.com/iai-callgrind/iai-callgrind/pull/407)): Update
  summary json schema v5.
* Update direct dependencies: `inferno`, `cc`, `clap`

### Deprecated

* ([#407](https://github.com/iai-callgrind/iai-callgrind/pull/407)): The methods
  `Callgrind::limits` and `Cachegrind::limits` to specify soft limits are now
  deprecated. Use `Callgrind::soft_limits`, `Cachegrind::soft_limits` instead
  for soft limits or `Callgrind::hard_limits`, `Cachegrind::hard_limits` for
  hard limits.

### Fixed

* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): Running
  dhat in `ad-hoc` mode exited with error due to a failed assertion. Parsing the
  log file in ad-hoc mode now succeeds.
* ([#406](https://github.com/iai-callgrind/iai-callgrind/pull/406)): Wrong error
  message `Failed to split callgrind args` when parsing of `--cachegrind-args`,
  `--dhat-args`, ... It's changed to `Failed to split args`.

## [0.15.2] - 2025-07-03

### Added

* ([#387](https://github.com/iai-callgrind/iai-callgrind/pull/387)): Calculate
  the cache miss rates (`I1 Miss Rate`, `LLi Miss Rate`, `D1 Miss Rate`, `LLd
  Miss Rate`, `LL Miss Rate`) and hit rates (`L1 Hit Rate`, `LL Hit Rate`, `RAM
  Hit Rate`). These new metrics are not part of the default output. Show them on
  demand when configured in `Callgrind::format` and/or `Cachegrind::format`.

### Changed

* Bump `summary.v4.schema.json` to `summary.v5.schema.json`
* Update direct dependencies: `schemars`, `indexmap` and transitive dependencies
* ([#395](https://github.com/iai-callgrind/iai-callgrind/pull/395)): Rename `L2
  Hits` to `LL Hits` to be closer to the original naming scheme in the callgrind
  documentation. This also eliminates differences to the naming of other metrics
  like `LLi Miss Rate`, ...
* ([#396](https://github.com/iai-callgrind/iai-callgrind/pull/396)): Append
  `(old)` in the terminal output of baselines with the same name to clarify
  which is the new and the old run.

### Fixed

* Fix broken links in the documentation/guide

## [0.15.1] - 2025-06-23

### Changed

* chore(deps): Update schemars from `0.9` -> `1.0`. Update the
  `summary.v4.schema.json` file to the new format.

### Fixed

* ([#382](https://github.com/iai-callgrind/iai-callgrind/pull/382)): The version
  for the summary json schema submitted in the `summary.json` and in json output
  was the old `3` instead of `4`.
* ([#383](https://github.com/iai-callgrind/iai-callgrind/issues/383)): The
  cachegrind feature check for library benchmarks happened at compile time of
  the user instead of iai-callgrind's. The `cachegrind` feature did not work
  (for library benchmarks) and rust versions above `1.80` produce a compilation
  warning due to `unexpected-cfgs`.

## [0.15.0] - 2025-06-22

Support running cachegrind instead of callgrind or in addition to callgrind if
required. The change also allowed a more flexible way to run benchmarks with any
valgrind tool as default tool if wished so.

### Added

* ([#365](https://github.com/iai-callgrind/iai-callgrind/pull/365)): Adjustable
  metrics in the terminal output of callgrind
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): Support to
  run cachegrind instead of callgrind or in addition to callgrind. The
  `cachegrind` feature of iai-callgrind allows to switch between both tools in
  the `Cargo.toml` in a more permanent way. But, it is also possible to change
  the default tool to cachegrind (or any other valgrind tool) on the
  command-line with `--default-tool` option. The
  `LibraryBenchmarkConfig::default_tool` (`BinaryBenchmarkConfig::default_tool`)
  can be used in the benchmarks to selectively change the default tool. To be
  able to define cachegrind limits in the same way as `--callgrind-limits` to
  detect regressions, the `--cachegrind-limits` options was added.
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): In the same
  way as `--callgrind-args` can be used on the command-line the following
  options were added to pass arguments to any valgrind tool: `--valgrind-args`,
  `--cachegrind-args`, `--dhat-args`, `--memcheck-args`, `--helgrind-args`,
  `--drd-args`, `--massif-args`, `--bbv-args`
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): Added the
  command-line arguments `--tools` to run additional tools
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The new
  structs `Callgrind`, `Cachegrind`, `DHAT`, `Memcheck`, `Helgrind`, `DRD`,
  `Massif`, `BBV` replace the old more generic `Tool` to be able to specify tool
  specific options. These structs can be passed to
  `LibraryBenchmarkConfig::tool` and `BinaryBenchmarkConfig::tool`.
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): Adjustable
  metrics in the terminal output for all tools.

### Changed

* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The
  command-line argument name `--regressions` changed to `--callgrind-limits`.
  The `IAI_CALLGRIND_REGRESSIONS` environment variable changed to
  `IAI_CALLGRIND_CALLGRIND_LIMITS`.
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The summary
  summary json schema v3 `summary.v3.schema.json` was updated to v4
  `summary.v4.schema.json`
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): Ignore with
  a warning the arguments `--xtree-memory`, `--xtree-memory-file`,
  `--xtree-leak`, `--xtree-leak-file`
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): A small
  change in the regression summary at the end of the benchmark run: The tool is
  now printed along with the detected regression: `Callgrind: Instructions (132
  -> 195): +47.7273% exceeds limit of +0.00000%` instead of just `Instructions
  (132 -> 195): +47.7273% exceeds limit of +0.00000%`
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The
  comparison by id between benchmark functions now compares the metrics of all
  tools and not just callgrind.
* Update direct dependencies: `cc`, `syn`, `clap`, `cfg-if`, `bindgen`, `which`
  and all transitive dependencies to their latest possible versions.

### Removed

* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The
  following functions were removed `BinaryBenchmarkConfig::with_callgrind_args`,
  `BinaryBenchmarkConfig::raw_callgrind_args`,
  `BinaryBenchmarkConfig::callgrind_args` (now in `Callgrind::args`),
  `BinaryBenchmarkConfig::flamegraph` (now in `Callgrind::flamegraph`),
  `BinaryBenchmarkConfig::regression` (now in `Callgrind::regression`),
  `BinaryBenchmarkConfig::entry_point` (now in `Callgrind::entry_point`)
  `BinaryBenchmarkConfig::tools`,
  `BinaryBenchmarkConfig::tools_override`,
  `LibraryBenchmarkConfig::with_callgrind_args`,
  `LibraryBenchmarkConfig::raw_callgrind_args`,
  `LibraryBenchmarkConfig::callgrind_args`,
  `LibraryBenchmarkConfig::with_raw_callgrind_args`,
  `LibraryBenchmarkConfig::flamegraph`,
  `LibraryBenchmarkConfig::regression`,
  `LibraryBenchmarkConfig::entry_point`,
  `LibraryBenchmarkConfig::tools`,
  `LibraryBenchmarkConfig::tools_override`,
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The
  `Tool` struct was removed and replaced by the more specific `Callgrind`,
  `Cachegrind`, ... structs
* ([#372](https://github.com/iai-callgrind/iai-callgrind/pull/372)): The
  deprecated `black_box` function was removed. Use `std::hint::black_box`
  instead.

## [0.14.2] - 2025-06-04

### Added

* ([#356](https://github.com/iai-callgrind/iai-callgrind/issues/356)): Print a
  summary after a benchmark run with total/regressed benchmarks and total
  execution time. Thanks to @tgross35 for the feedback and format suggestion.

### Changed

* ([#361](https://github.com/iai-callgrind/iai-callgrind/pull/361)): Wrap long
  lines of `--help` output to 100 max bytes for better readability.
* ([#362](https://github.com/iai-callgrind/iai-callgrind/pull/362)): Return with
  exit code `3` on regression errors and `2` on command-line argument parsing
  errors.
* Update dependencies to latest possible versions

### Fixed

* ([#360](https://github.com/iai-callgrind/iai-callgrind/pull/360)): Stay closer
  to the rust format of `--list` and use `: benchmark` suffix instead of `:
  bench` when using `--list` to print a list of all benchmarks. Thanks to
  @gaetschwartz

## [0.14.1] - 2025-05-31

### Added

* ([#339](https://github.com/iai-callgrind/iai-callgrind/pull/339)): Implement
  the --list argument of the libtest harness to list all benchmarks instead of
  running any benchmarks.

### Changed

* ([#347](https://github.com/iai-callgrind/iai-callgrind/pull/347)): Update MSRV
  from `1.67.1` -> `1.74.1` and update (most) direct and indirect dependencies
  to their latest versions.

### Fixed

* ([#337](https://github.com/iai-callgrind/iai-callgrind/issues/337)): Fix
  `--regression` does not exit with error status when there are regressions.
  Also, `--regression-fail-fast` did not exit the benchmarks early on first
  encountered regression.
* ([#339](https://github.com/iai-callgrind/iai-callgrind/issues/339)): Fix tests
  fail when invoked with extra cargo (libtest harness) arguments like --list or
  --format, ...
* ([#341](https://github.com/iai-callgrind/iai-callgrind/issues/339)): Remove
  Quickstart from README and link instead to the quickstart in the guide.
* ([#347](https://github.com/iai-callgrind/iai-callgrind/pull/347)): deny:
  RUSTSEC-2025-0024: error[vulnerability]: crossbeam-channel: double free on
  Drop: Fixed by updating transient dependency `crossbeam-channel` to 0.5.15
* ([#347](https://github.com/iai-callgrind/iai-callgrind/pull/347)):
  RUSTSEC-2025-0024: deny: error[vulnerability]: Borsh serialization of HashMap
  is non-canonical: Fixed by updating transient `hashbrown` dependency to 0.15.3

## [0.14.0] - 2024-10-04

This release adds full support for multi-threaded and multi-process
applications.

When upgrading from a previous release of Iai-Callgrind you might experience
changes in the metrics without having changed the benchmarks themselves. The
`summary` line in callgrind output files turned out to be buggy and unreliable
if client requests are used, so Iai-Callgrind now parses the `totals` instead.
The `totals` might differ slightly from the `summary` and cause the difference
in the displayed metrics. You might also see changes in the metrics because of
the changed default values for some of the valgrind arguments. Iai-Callgrind
changed the following default valgrind/callgrind arguments for each benchmark
run:

* `--separate-threads=no` -> `--separate-threads=yes`
* `--trace-children=no` -> `--trace-children=yes`
* `--fair-sched=no` -> `--fair-sched=try`

You can now run the `setup` in binary benchmarks in parallel to the `Command`
for flexible benchmarking of client/server architectures.

The MSRV has changed from `1.66.0` -> `1.67.1`.

If not stated otherwise the changes below were introduced in
[#263](https://github.com/iai-callgrind/iai-callgrind/pull/263).

### Added

* Support for benchmarks of multi-threading and multi-process applications by
  implementing the correct handling of the valgrind `--trace-children` and
  callgrind `--separate-threads` command line options. Per default only the
  total over all subprocesses and threads is calculated and shown. But, each
  thread and subprocess can be displayed with the new
  `OutputFormat::show_intermediate` option.
* Support for the callgrind command line arguments `--dump-every-bb`,
  `--dump-before`, `--dump-after` which create parts. These parts are now
  correctly summarized in the total and the metrics of each part can be shown
  with `OutputFormat::show_intermediate`.
* Added `OutputFormat` which can be used in
  `LibraryBenchmarkConfig::output_format` and
  `BinaryBenchmarkConfig::output_format` to change some of the behaviour of the
  default terminal output (but not json output)
* Sometimes callgrind creates empty files, so we're cleaning them up now after
  each benchmark run.
* ([#256](https://github.com/iai-callgrind/iai-callgrind/pull/256)) and
  ([#279](https://github.com/iai-callgrind/iai-callgrind/pull/279)): Support
  running setup in parallel and add possibility to delay the `Command`. Thanks
  to @hargut for #256
* ([#288](https://github.com/iai-callgrind/iai-callgrind/pull/288)): Added an
  option `OutputFormat::show_grid` to show grid/guiding lines which can help
  reading the terminal output if running benchmarks with multiple
  threads/subprocesses/tools.
* The method `BinaryBenchmarkConfig::with_callgrind_args` was added to match the
  constructors of the `LibraryBenchmarkConfig`.
* The methods `BinaryBenchmarkConfig::valgrind_args` and
  `LibraryBenchmarkConfig::valgrind_args` are introduced to be able to pass
  valgrind core arguments to all tools.

### Changed

* All tools are now per default run with `--trace-children=yes` and
  `--fair-sched=try`. In addition, callgrind is run with
  `--separate-threads=yes`. These default arguments can be changed in
  `Tool::args` or `LibraryBenchmarkConfig::callgrind_args`,
  `BinaryBenchmarkConfig::callgrind_args`.
* The file naming scheme was adjusted to include the pids in case of
  multi-process benchmarks, the parts in case of callgrind command-line
  arguments which create multiple parts and threads in case of multiple threads.
  This change is backwards compatible to the file naming scheme of previous
  Iai-Callgrind releases for all tools but `exp-bbv`.
* Error metrics from tools like drd, helgrind and memcheck are now listed and
  compared like the other metrics in a vertical format. For example

  ```text
  ======= DRD ===============================================================
  Errors:                           0|0               (No change)
  Contexts:                         0|0               (No change)
  Suppressed Errors:                0|0               (No change)
  Suppressed Contexts:              0|0               (No change)
  ```

* ([#263](https://github.com/iai-callgrind/iai-callgrind/pull/263)) and
  ([#288](https://github.com/iai-callgrind/iai-callgrind/pull/288)): Increase
  the field width by 3 bytes and the space for metrics by 5 on each side of the
  comparison so that the value of `u64::MAX` fits into the terminal output
  without messing up the side-by-side layout.
* The `LibraryBenchmarkConfig::truncate_description`,
  `BinaryBenchmarkConfig::truncate_description` methods have been moved to
  `OutputFormat::truncate_description`
* In the presence of multiple processes the DHAT metrics are now summarized and
  shown in a total in the same way as the metrics of callgrind and the other
  tools.
* Bump the summary json schema to v3 in
  `iai-callgrind-runner/schemas/summary.v3.schema.json`
* Various prs: Update locked direct dependencies:
    * `anyhow` -> 1.0.89
    * `cc` -> 1.1.25
    * `indexmap` -> 2.6.0
    * `itertools` -> 0.13.0
    * `once_cell` -> 1.20.1
    * `regex` -> 1.11.0
    * `serde_json` -> 1.0.128
    * `serde` -> 1.0.210
    * `syn` -> 2.0.79
    * `tempfile` -> 3.13.0
* ([#288](https://github.com/iai-callgrind/iai-callgrind/pull/288)): The default
  include path for the valgrind headers has changed to `/usr/local/include` on
  freebsd instead of `/usr/local`.
* ([#289](https://github.com/iai-callgrind/iai-callgrind/pull/289)): Update
  `derive_more` -> `1.0` in `Cargo.toml` but not in lock file.
* ([#293](https://github.com/iai-callgrind/iai-callgrind/pull/293)): Update MSRV
  from `1.66.0` -> `1.67.1`
* ([#296](https://github.com/iai-callgrind/iai-callgrind/pull/296)): Update
  locked transitive dependencies.

### Deprecated

* The following methods were renamed and deprecate the old method name:
    * `LibraryBenchmarkConfig::raw_callgrind_args` ->
      `LibraryBenchmarkConfig::callgrind_args`,
    * `LibraryBenchmarkConfig::with_raw_callgrind_args` ->
      `LibraryBenchmarkConfig::with_callgrind_args`
    * `BinaryBenchmarkConfig::raw_callgrind_args` ->
      `BinaryBenchmarkConfig::callgrind_args`
* The method `LibraryBenchmarkConfig::raw_callgrind_args_iter` was deprecated
  since it is the same as `LibraryBenchmarkConfig::callgrind_args`.

### Removed

* Iai-Callgrind doesn't support combined dumps via `--combine-dumps` anymore.
* The `Tool::outfile_modifier` method was removed. The `%p` modifier for
  valgrind output and log files is now applied automatically when using the
  `--trace-children=yes` command line argument.
* The output and log file paths in the terminal output were removed.

### Fixed

* When extracting the metrics from callgrind output files, the totals line is
  now prioritized over the summary line. The summary line has bugs and reports
  wrong costs if callgrind client requests are used. The totals are unaffected
  by client requests and report the correct costs. This change is mostly
  internal but might introduce some (small) changes in the metrics reported by
  Iai-Callgrind.
* The error metrics of drd, helgrind and memcheck were only shown correctly if
  they consisted of a single digit.
* ([#297](https://github.com/iai-callgrind/iai-callgrind/pull/297)): Added the
  derive `Clone` impl for `iai_callgrind::LibraryBenchmarkConfig`
* ([#300](https://github.com/iai-callgrind/iai-callgrind/pull/300)):
  `library_benchmark_group!` was private but the expanded mod should be public
  Thanks to @ryanpeach

## [0.13.4] - 2024-09-12

### Changed

* ([#264](https://github.com/iai-callgrind/iai-callgrind/pull/264)): Migrate
  from unmaintained proc-macro-error to proc-macro-error2 due to
  <https://rustsec.org/advisories/RUSTSEC-2024-0370>. This also removes the
  duplicate dependency on `syn v2` and `syn v1`.

## [0.13.3] - 2024-09-05

The installation of `iai-callgrind-runner` with `cargo install` did not use the
cache when trying to install the same version again and acted as if `cargo
install --force` was given which leads to longer installation times in case the
binary was already installed. See this
[issue](https://github.com/iai-callgrind/iai-callgrind/issues/260) for more
details.

This problem is fixed in this and the following releases, but not in older
versions of `iai-callgrind-runner`. Please use
[`binstall`](https://github.com/cargo-bins/cargo-binstall) instead of `cargo
install` for these versions if installation time is a concern. `binstall` seems
to correctly recognize the same installation and does not install
`iai-callgrind-runner` from scratch again.

### Added

* ([#254](https://github.com/iai-callgrind/iai-callgrind/pull/254)): Added the
  option to switch off the entry point `EntryPoint::None` or use a custom entry
  point (`EntryPoint::Custom`). The default entry point stays the same and is
  the toggle Iai-Callgrind sets with `--toggle-collect` to the benchmark
  function.

### Changed

* ([#254](https://github.com/iai-callgrind/iai-callgrind/pull/254)): Due to the
  changes required to handle the different entry points options, the flamegraphs
  created in binary benchmarks and flamegraphs from library benchmarks with
  `EntryPoint::None` include all events, not only the events from `main`
  downwards.

### Fixed

* ([#261](https://github.com/iai-callgrind/iai-callgrind/pull/261)):
  Reinstalling iai-callgrind-runner with `cargo install` when it was already
  installed acted as if `cargo install --force` was given.

## [0.13.2] - 2024-09-03

### Fixed

* ([#252](https://github.com/iai-callgrind/iai-callgrind/pull/252)): When using
  callgrind client requests like `start_instrumentation`, `stop_instrumentation`
  together with `--collect-at-start=no` then all metrics were zero. Thanks to
  @hargut
* ([#257](https://github.com/iai-callgrind/iai-callgrind/pull/257)): A small
  cosmetic fix for the factor in the benchmark output if it was negative
  infinite. `[++-inf+++]` was changed to `[---inf---]`.
* ([#258](https://github.com/iai-callgrind/iai-callgrind/pull/258)): The
  `teardown` function of a `library_benchmark_group!` was only executed if a
  `setup` function was present, too.

## [0.13.1] - 2024-08-28

### Changed

* Updated locked non-development dependencies:
    * `cc`: 1.1.13 -> 1.1.15
    * `quote`: 1.0.36 -> 1.0.37
    * `serde`: 1.0.208 -> 1.0.209
    * `serde_json`: 1.0.126 -> 1.0.127
    * `syn`: 2.0.75 -> 2.0.76

### Fixed

* ([#248](https://github.com/iai-callgrind/iai-callgrind/pull/248)): If the
  Command's path was a simple command name like `echo`, `cat`, the path was
  interpreted as relative path instead of searched in the `$PATH`. Relative
  paths like `./echo` are now interpreted as relative to the current directory.
  If running the Command in a Sandbox, this is the root directory of the
  Sandbox. Otherwise, it is the directory which is set by cargo bench.

## [0.13.0] - 2024-08-19

!!! __IMPORTANT__ The default to run binary benchmarks in a sandbox has been
changed from `true` to `false`. The `setup` and `teardown` of the
`binary_benchmark_group!` are not executed in the sandbox anymore !!!

The way to set up binary benchmarks has completely changed and has been
rewritten from scratch! The api for binary and library benchmarks is now
consistent and most features from library benchmarks which were missing for
binary benchmarks are now available in binary benchmarks, too. For example
comparison of benchmarks by id. If you are using library benchmarks but not
binary benchmarks, this release doesn't change much. There are no breaking
changes for library benchmarks, and you can jump right to the changes section of
this release. Otherwise, here's a small introduction to the new api and the
changes for binary benchmarks.

There are a lot of advantages for you and honestly for us, too, because we don't
have to maintain two completely different apis. Binary benchmarks and library
benchmarks can now be written in a similar fashion what makes writing benchmarks
for a crate's binaries just easier and faster. No need to learn a completely
different api if you already used library benchmarks and vice versa! Also, the
feature set between library benchmarks and binary benchmarks diverged over time.
For example comparison by id of benchmarks within the same group was available
in library benchmarks via the `library_benchmark_group!` macro but not in binary
benchmarks. Such differences are gone, now. Also, if you find out the new
`#[binary_attribute]` does not provide you with the same power as the old
builder api, you can still use a low level api and can even intermix the two
styles. The new low level api is more intuitive than the old builder api, just
more powerful and mirrors the `binary_benchmark` attribute as much as possible.

For example if the crate's binary is named `my-binary`:

```rust
use iai_callgrind::{binary_benchmark, binary_benchmark_group};

#[binary_benchmark]
#[bench::some_id("foo")]
fn bench_binary(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = some_group;
    benchmarks = bench_binary
);
```

can also be written with the low level api

```rust
use iai_callgrind::{binary_benchmark_group, BinaryBenchmark, Bench};

binary_benchmark_group!(
    name = low_level;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            .binary_benchmark(
                BinaryBenchmark::new("bench_binary")
                    .bench(
                        Bench::new("some_id").command(
                            iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
                                .arg("foo")
                        )
                    )
            )
    }
);
```

However, the first method using the `#[binary_benchmark]` attribute is the new
and recommended way to set up binary benchmarks, since it is more descriptive and
concise, especially with a lot of benchmarks. And, if you need to set up only
some benchmarks in a way which stretches the `#[binary_benchmark]` attribute to
its limits, you can intermix both styles and switch to the low level api in a
few steps:

```rust
use iai_callgrind::{
     binary_benchmark, binary_benchmark_attribute, binary_benchmark_group, BinaryBenchmark, Bench
};

// No need to translate this into the low level api. Just keep it as it is and
// have a look at the usage of the `binary_benchmark_attribute!` macro below
#[binary_benchmark]
#[bench::some_id("foo")]
fn attribute_benchmark(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = low_level;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            // Add the benchmark function `attribute_benchmark` annotated with the
            // #[binary_benchmark] attribute with the `binary_benchmark_attribute!` macro
            .binary_benchmark(binary_benchmark_attribute!(attribute_benchmark))
            // For the sake of simplicity, assume this would be the benchmark you
            // were not able to setup with the attribute
            .binary_benchmark(
                BinaryBenchmark::new("low_level_benchmark")
                    .bench(
                        Bench::new("some_id").command(
                            iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
                                .arg("bar")
                                .build()
                        )
                    )
            )
    }
);
```

In contrast to binary benchmarks, not much has changed for library benchmarks.
If you're just looking for the changes in library benchmarks, the changes for
library benchmarks have been marked specifically. The changes below were
introduced in ([#229](https://github.com/iai-callgrind/iai-callgrind/pull/229)).

### Added

* *(library/binary benchmarks)*: Hard limit of 5000 bytes for the `DESCRIPTION`
  in the benchmark output (`benchmark_file::group::function_name
  id:DESCRIPTION`). The description is passed from `iai-callgrind-macros`
  through `iai-callgrind` to the `iai-callgrind-runner`. The sole purpose of the
  description is to show the context in which the benchmark is executed. In the
  case of a very large input and gigabytes of data, the description was trimmed
  very late in the `iai-callgrind-runner` and kept in memory for a rather long
  time. A hard limit of 5000 bytes very early in `iai-callgrind-macros` reduces
  memory usage without destroying the purpose of the description.
* `binary_benchmark_group!` macro: The `compare_by_id` argument has been added
  and works the same way as the `compare_by_id` argument in the
  `library_benchmark_group`.
* *(binary benchmarks)*: `main!` macro: The `setup` and `teardown` arguments
  were added. The `setup` argument is run before all benchmarks in the binary
  benchmark groups and `teardown` after all benchmarks.
* `#[binary_benchmark]` attribute: This attribute needs to be specified on a
  function which is used in the `benchmarks` argument of the
  `binary_benchmark_group!` macro. The attribute takes the `config`, `setup` and
  `teardown` parameters.
* `#[bench]`, `#[benches]` attributes for `#[binary_benchmark]`: These
  attributes serve the same purpose as the `#[bench]` and `#[benches]` attribute
  in a `#[library_benchmark]` annotated function and take the same parameters.
* *(library/binary benchmarks)*: The `setup`, `teardown` of the `main!` and
  `library_benchmark_group!` (`binary_benchmark_group!`) macros now have access
  to the environment variables of the `LibraryBenchmarkConfig`
  (`BinaryBenchmarkConfig`) at the respective level.

### Changed

* The default to run binary benchmarks in a sandbox has been changed from `true`
  to `false`. Also, the sandbox is now set up for each benchmark individually
  instead of once per group and can now be configured in
  `BinaryBenchmarkConfig::sandbox`.
* `binary_benchmark_group!` macro: The `setup` and argument now takes an
  expressions (including function calls) and is run __once__ before all
  benchmarks in this group. The `teardown` argument also takes an expression and
  is run __once__ after all benchmarks in this group. __once__ in bold because
  previously `setup` and `teardown` were run for each benchmark in this group.
  These functions are also not executed in the sandbox anymore.

### Removed

* `binary_benchmark_group!` macro: The `before` and `after` arguments have been
  removed. The possibility to benchmark the `setup` and `before` functions via
  the `bench = bool` argument has been removed, since it caused more problems
  than it solved. In the rare case, you really need to benchmark `setup` or
  `teardown` you can use the functionality of library benchmarks.
* `BinaryBenchmarkConfig::entry_point`: This method was removed. Using
  `BinaryBenchmarkConfig::raw_callgrind_args(["toggle-collect=ENTRY_POINT"])` is
  the more idiomatic and less confusing way to achieve the same.

## [0.12.3] - 2024-08-09

### Added

* ([#221](https://github.com/iai-callgrind/iai-callgrind/pull/221)): Add the
  `LibraryBenchmarkConfig::truncate_description` and
  `BinaryBenchmarkConfig::truncate_description` method to be able to adjust
  the truncation behaviour of the `DESCRIPTION` (as in
  `benchmark_file::group::function_name id:DESCRIPTION`) in the benchmark
  output.

### Changed

* ([#221](https://github.com/iai-callgrind/iai-callgrind/pull/221)): Slightly
  increase the default truncation length of the description in the benchmark
  output to 50 ascii characters.
* Update locked non-development dependencies:
    * `tempfile`: 3.11.0 -> 3.12.0
    * `serde`: 1.0.204 -> 1.0.205

## [0.12.2] - 2024-08-06

### Added

* ([#210](https://github.com/iai-callgrind/iai-callgrind/pull/210)): Add the
  `file` parameter to the `#[benches]` attribute to support creation of
  benchmarks from the specified file. Each line of this file represents a new
  benchmark and the read line is passed to the benchmark function or if the
  `setup` parameter is also present to the `setup` function.
* ([#211](https://github.com/iai-callgrind/iai-callgrind/pull/211)): Add support
  for a `setup` and `teardown` function in the `library_benchmark_group` and
  `main` macro. The group `setup` function is run before any benchmark of this
  group and the `teardown` function after all benchmarks of the same group.
  Similarly, the `setup` function of the `main` macro is run before any
  benchmark group and the `teardown` function after all benchmarks.

### Changed

* Update locked non-development dependencies:
    * `regex`: 1.10.5 -> 1.10.6
    * `tempfile`: 3.10.1 -> 3.11.0
    * `serde_json`: -> 1.0.121 -> 1.0.122
    * `indexmap`: 2.2.6 -> 2.3.0

### Fixed

* The library documentation in parts still mentioned
  `EventKind::EstimatedCycles` as default regression kind instead of
  `EventKind::Ir`. This default has changed in `v0.11.0`.

## [0.12.1] - 2024-07-31

### Changed

* ([#212](https://github.com/iai-callgrind/iai-callgrind/pull/212)): Update
  transitive dependency `bytemuck` 1.15.0 (yanked) -> 1.16.3
* Update other locked non-development dependencies:
    * `cc`: 1.1.5 -> 1.1.7,
    * `serde_json`: 1.0.120 -> 1.0.121

## [0.12.0] - 2024-07-24

### Added

* ([#160](https://github.com/iai-callgrind/iai-callgrind/pull/160)): Add
  `--separate-targets` (env: `IAI_CALLGRIND_SEPARATE_TARGETS`). Using this
  option causes the compilation target to be included in the iai-callgrind
  output directory tree to mitigate issues when running benchmarks on multiple
  targets. For example, instead of having all output files under `target/iai`,
  using this option puts all files under the directory
  `target/iai/x86_64-unknown-linux-gnu` if running the benchmarks on the
  `x86_64-unknown-linux-gnu` target.
* ([#188](https://github.com/iai-callgrind/iai-callgrind/pull/188)): Add the
  option `--home` (env: `IAI_CALLGRIND_HOME`) to be able to change the default
  home directory `target/iai`.
* ([#192](https://github.com/iai-callgrind/iai-callgrind/pull/192)): The
  `#[bench]` attribute now accepts a `setup` parameter similarly to the
  `#[benches]` attribute. The `#[bench]` and `#[benches]` attribute accept a
  new `teardown` parameter. The `teardown` function is called with the return
  value of the benchmark function. The `#[library_benchmark]` attribute now
  accepts a global `setup` and `teardown` parameter which are applied to all following
  `#[bench]` and `#[benches]` attributes if they don't specify one of these
  parameters themselves.
* ([#194](https://github.com/iai-callgrind/iai-callgrind/pull/194)): Add
  `--nocapture` (env: `IAI_CALLGRIND_NOCAPTURE`) option to tell iai-callgrind to
  not capture `callgrind` terminal output of benchmark functions. For all
  possible values see the `README`.
* ([#201](https://github.com/iai-callgrind/iai-callgrind/pull/201)): Add support
  for generic benchmark functions fixing #198 (Generic bench arguments cause
  compilation failure).

### Changed

* Update non-development locked dependencies: `syn` -> 2.0.72, `cc` -> 1.1.5, `serde` -> 1.0.204
* Update minimal version of `syn` -> 2.0.32
* ([#201](https://github.com/iai-callgrind/iai-callgrind/pull/201)): The
  `BinaryBenchmarkConfig::entry_point` and `Run::entry_point` functions now use
  glob patterns as argument with `*` as placeholder for any amount of
  characters.
* ([#203](https://github.com/iai-callgrind/iai-callgrind/pull/203)): Improve
  error messages during the initialization phase of the `iai-callgrind-runner`,
  get rid of a lot of unwraps and include a solution hint. These errors mainly
  happen if the `iai-callgrind` library has a different version than the
  `iai-callgrind-runner` binary.

### Fixed

* ([#192](https://github.com/iai-callgrind/iai-callgrind/pull/192)): Fix a
  wrongly issued compiler error when the setup parameter was specified before
  the args parameter and the number of elements of the args parameter did not
  match the number of arguments of the benchmark function.
* ([#192](https://github.com/iai-callgrind/iai-callgrind/pull/192)): Fix the
  error span of wrong user supplied argument types or wrong number of arguments.
  The compiler errors now point to the exact location of any wrong arguments
  instead of the generic call-site of the `#[library_benchmark]` attribute. If
  there is a setup function involved, we leave it to the rust compiler to point
  to the location of the setup function and the wrong arguments.

## [0.11.1] - 2024-07-05

### Changed

* ([#169](https://github.com/iai-callgrind/iai-callgrind/pull/169)): Clearify
  documentation about the scope of uniqueness of benchmark ids. Thanks to @peter-kehl
* ([#175](https://github.com/iai-callgrind/iai-callgrind/pull/175)): Mark
  iai-callgrind build dependencies required only by the `client_request_defs`
  feature as optional. Solve cargo's `--check-cfg` warnings if currently active
  rust version is `>= 1.80.0`. Thanks to @DaniPopes
* Update some locked dependencies

## [0.11.0] - 2024-05-09

The default `EventKind` for `RegressionConfig` and `FlamegraphConfig` changed,
to `EventKind::Ir` so, if you're updating from a previous version of
`iai-callgrind`, please read carefully!

### Added

* ([#71](https://github.com/iai-callgrind/iai-callgrind/issues/71)): Add a DHAT
  cost summary similar to the summary of callgrind events in the benchmark run
  output. Thanks to @dewert99.
* ([#80](https://github.com/iai-callgrind/iai-callgrind/issues/80)): Add
  pre-built `iai-callgrind-runner` binaries for most valgrind supported targets
  to the github release pages. `iai-callgrind-runner` can now also be installed
  with `cargo binstall`.
* ([#88](https://github.com/iai-callgrind/iai-callgrind/issues/88)): Support
  filtering benchmarks by name. This is a command-line option only and the
  filter can be given as positional argument in `cargo bench -- FILTER`.
  Specifying command-line arguments in addition to the `FILTER` still works.
* ([#144](https://github.com/iai-callgrind/iai-callgrind/pull/144)): Verify
  compatibility with latest valgrind release 3.23.0 and update client requests
  to newly supported target arm64/freebsd.
* ([#152](https://github.com/iai-callgrind/iai-callgrind/pull/152)): Support
  comparison of benches in library benchmark functions by id.
* ([#158](https://github.com/iai-callgrind/iai-callgrind/pull/152)): Support
  environment variable `IAI_CALLGRIND_<TRIPLE>_VALGRIND_INCLUDE` with `<TRIPLE>`
  being the hosts target triple. This variable takes precedence over the more
  generic `IAI_CALLGRIND_VALGRIND_INCLUDE` environment variable. Thanks to
  @qRoC

### Changed

* ([#94](https://github.com/iai-callgrind/iai-callgrind/issues/94)): Support
  running `iai-callgrind` benchmarks without cache simulation
  (`--cache-sim=no`). Previously, specifying this option emitted a warning. Note
  that running the benchmarks with `--cache-sim=no` implies that there is also
  no estimated cycles calculation.
* ([#106](https://github.com/iai-callgrind/iai-callgrind/pull/106)): Due to
  [#94](https://github.com/iai-callgrind/iai-callgrind/issues/94), the
  default `EventKind` for `RegressionConfig` and `FlamegraphConfig` changed from
  `EventKind::EstimatedCycles` to `EventKind::Ir`.
* Updated locked dependencies to their most recent version
* Due to backwards incompatible changes to the summary schema the schema version
  was updated v1 -> v2. The current schema file is stored in
  `iai-callgrind-runner/schemas/summary.v2.schema.json`

### Fixed

* ([#86](https://github.com/iai-callgrind/iai-callgrind/pull/86)): Fix
  positional arguments meant as filter as in `cargo bench -- FILTER` cause
  `iai-callgrind` to crash.
* ([#110](https://github.com/iai-callgrind/iai-callgrind/pull/110)): Fix example
  in README. Thanks to @jembishop
* ([#145](https://github.com/iai-callgrind/iai-callgrind/pull/145)): Fixed an
  error on freebsd when copying fixtures in binary benchmarks.

## [0.10.2] - 2024-01-25

### Changed

* Update locked dependencies

### Fixed

* ([#84](https://github.com/iai-callgrind/iai-callgrind/pull/84)): Fix an error
  when `--load-baseline` loads the dataset from the `--baseline` argument. This
  error led to a comparison of the `--baseline` dataset with itself.

## [0.10.1] - 2024-01-22

### Changed

* Update env_logger and which dependencies in Cargo.toml
* Update locked dependencies

### Fixed

* ([#81](https://github.com/iai-callgrind/iai-callgrind/pull/81)): Fix security
  advisory RUSTSEC-2024-0006 of shlex dependency and update shlex to 1.3.0. Use
  `shlex::try_join` instead of deprecated `shlex::join`.

## [0.10.0] - 2024-01-09

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

## [0.9.0] - 2023-12-09

### Added

* ([#31](https://github.com/iai-callgrind/iai-callgrind/issues/31)): Machine
  readable output. This feature adds an environment variable
  `IAI_CALLGRIND_SAVE_SUMMARY` and command line argument `--save-summary` to
  create a `summary.json` next to the usual output files of a benchmark which
  contains all the terminal output data and more in a machine-readable output
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

## [0.8.0] - 2023-11-10

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

## [0.7.3] - 2023-10-24

### Changed

* Update repository to use github organization `iai-callgrind/iai-callgrind`
* Lower the locked inferno dependency to `0.11.12` to workaround yanked `ahash`
  version `0.8.3`

## [0.7.2] - 2023-10-18

### Added

* ([#23](https://github.com/iai-callgrind/iai-callgrind/issues/23)): Create
  regular and differential flamegraphs from callgrind output.

### Fixed

* ([#22](https://github.com/iai-callgrind/iai-callgrind/pull/22)): Clearify how
  to update iai-callgrind-runner
* Some small fixes of parsing callgrind output files in the event that no
  records are present.

## [0.7.1] - 2023-09-27

### Fixed

* ([#20](https://github.com/iai-callgrind/iai-callgrind/issues/20)): Clearing the
  environment variables with `env_clear` may break finding valgrind.

## [0.7.0] - 2023-09-21

The old api to set up library benchmarks using only the `main!` macro is deprecated and was removed.
See the [README](./README.md) for a description of the new api.

Also, the api to set up binary benchmarks only with the `main!` macro is now deprecated and was
removed. Please use the builder api using the `binary_benchmark_groups!` and `Run`. The old binary
benchmark api lacked the rich possibilities of the builder api and maintaining two such different
apis adds a lot of unnecessary complexity.

Additionally, the scheme to set up binary benchmarks and specifying configuration options was
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
  (`#[library_benchmark]`) based api to set up library benchmarks. Also, bring the library benchmark
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

## [0.6.2] - 2023-09-01

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

## [0.6.1] - 2023-08-25

### Fixed

* ([#4](https://github.com/iai-callgrind/iai-callgrind/issues/4)): The destination
  directory of iai callgrind output files changes from `/workspace/$CARGO_PKG_NAME/target/iai` to
  `/workspace/target/iai/$CARGO_PKG_NAME` and respects the `CARGO_TARGET_DIR` environment variable

## [0.6.0] - 2023-08-20

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

## [0.5.0] - 2023-08-07

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

## [0.4.0] - 2023-07-19

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

## [0.3.1] - 2023-03-13

### Added

* Add output of Callgrind at `RUST_LOG=info` level but also more debug and trace output.

### Fixed

* The version mismatch check should cause an error when the library version is < 0.3.0

## [0.3.0] - 2023-03-13

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

* A cargo filter argument which is a positional argument resulted in the runner to crash.

## [0.2.0] - 2023-03-10

This version is mostly compatible with `v0.1.0` but needs some additional setup. See
[Installation](README.md#installation) in the README. Benchmarks created with `v0.1.0` should not
need any changes but can maybe be improved with the additional features from this version.

### Changed

* The repository layout changed and this package is now separated in a library
  (iai-callgrind) with the main macro and the black_box and the binary package (iai-callgrind-runner)
  with the runner needed to run the benchmarks
* It's now possible to pass additional arguments to callgrind
* The output of the collected event counters and metrics has changed
* Other improvements to stabilize the metrics across different systems

## [0.1.0] - 2023-03-08

### Added

* Initial migration from Iai
