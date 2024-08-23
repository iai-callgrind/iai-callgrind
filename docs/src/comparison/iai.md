# Comparison of Iai-Callgrind with Iai

This is a comparison with [Iai](https://github.com/bheisler/iai), from which
Iai-Callgrind is forked over a year ago.

Iai-Callgrind Pros:

* Iai-Callgrind is actively maintained.

* The benchmark api of Iai-Callgrind is simple, intuitive and allows for a much
  more concise and clearer structure of benchmarks.

* More stable metrics because the benchmark function is virtually encapsulated
  by Callgrind and separates the benchmarked code from the surrounding code.

* Iai-Callgrind excludes setup code from the metrics natively.

* The Callgrind output files are much more focused on the benchmark function and
  the function under test than the Cachegrind output files that Iai produces.
  The calibration run of Iai only sanitized the visible summary output but not
  the metrics in the output files themselves. So, the output of `cg_annotate`
  was still cluttered by the initialization code, setup functions and metrics.

* Changes to the library of Iai-Callgrind have almost never an influence on the
  benchmark metrics, since the actual runner (`iai-callgrind-runner`) and thus
  `99%` of the code needed to run the benchmarks is isolated from the
  benchmarks by an independent binary. In contrast to the library of Iai which
  is compiled together with the benchmarks.

* Iai-Callgrind has functionality in place that provides a more constant
  environment, like the `Sandbox` and clearing environment variables.

* Supports running other Valgrind Tools, like DHAT, Massif etc.

* Comparison of benchmark functions.

* Iai-Callgrind can be configured to check for performance regressions.

* A complete implementation of Valgrind Client Requests is available in
  Iai-Callgrind itself.

* Comparison of benchmarks to baselines instead of only to `.old` files.

* Iai-Callgrind natively supports benchmarking binaries.

* Iai-Callgrind can print machine-readable output in `.json` format.

I don't see any downside in using Iai-Callgrind instead of Iai.
