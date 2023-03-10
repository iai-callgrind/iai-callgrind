# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [0.2.0] 2023-03-10

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

### [0.1.0] 2023-03-08

### Added

* Initial migration from Iai
