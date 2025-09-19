# Comparison of Gungraun with Iai

This is a comparison with [Iai](https://github.com/bheisler/iai). There is no
known downside in using Gungraun instead of Iai. Although the original idea
of Iai will always be remembered, Gungraun has surpassed Iai over the years
in functionality, stability and flexibility.

Gungraun Pros:

* Gungraun is actively maintained.

* The user interface and benchmarking api of Gungraun is simple, intuitive
  and allows for a much more concise and clearer structure of benchmarks.

* Gungraun excludes setup code from the metrics of interest natively. The
  metrics are more stable because the benchmark function is virtually
  encapsulated by Callgrind and separates the benchmarked code from the
  surrounding code.

* Full support for benchmarking
  [multi-threaded/multi-process](../benchmarks/library_benchmarks/threads_and_subprocesses.md)
  functions/binaries.

* Can still run [Cachegrind](../cachegrind.md) but with a real one-shot
  implementation using client requests instead of a calibration run.

* Support of memory profiling with `DHAT` and `Massif`.

* Running error checking [valgrind tools](../tools.md) is a few keystrokes away
  if you really need them.

* The Callgrind output files are much more focused on the benchmark function and
  the function under test than the Cachegrind output files that Iai produces.
  The calibration run of Iai only sanitized the visible summary output but not
  the metrics in the output files themselves. So, the output of `cg_annotate`
  was still cluttered by the initialization code, setup functions and metrics.

* Changes to the library of Gungraun have almost never an influence on the
  benchmark metrics, since the actual runner (`iai-callgrind-runner`) and thus
  `99%` of the code needed to run the benchmarks is isolated from the
  benchmarks by an independent binary. In contrast to the library of Iai which
  is compiled together with the benchmarks.

* Gungraun has functionality in place that provides a constant and
  reproducible benchmarking environment, like the
  [`Sandbox`](../benchmarks/binary_benchmarks/configuration/sandbox.md) and
  clearing environment variables.

* Customizable output format to be able to show [all
  Callgrind/Cachegrind/DHAT/...](../benchmarks/library_benchmarks/configuration/output_format.md)
  metrics or only the set of metrics you're interested in.

* [Comparison by id](../benchmarks/library_benchmarks/compare_by_id.md) of
  benchmark functions.

* Gungraun can be configured to check for [performance
  regressions](../regressions.md).

* Ships with a complete implementation of [Valgrind Client
  Requests](../client_requests.md)

* Comparison of benchmarks to [baselines](../cli_and_env/baselines.md) instead
  of only to `.old` files.

* Natively supports [benchmarking binaries](../benchmarks/binary_benchmarks.md).

* Gungraun can print and/or save [machine-readable
  output](../cli_and_env/output/machine_readable.md) in `.json` format.

* Fixed the wrong labeling of `L1 Accesses`, ... to `L1 Hits`, ...
