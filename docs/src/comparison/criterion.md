# Comparison of Iai-Callgrind with Criterion-rs

This is a comparison with
[Criterion-rs](https://github.com/bheisler/criterion.rs?tab=readme-ov-file) but
some of the points in Pros and Cons also apply to other wall-clock time based
benchmarking frameworks.

Iai-Callgrind Pros:

* Iai-Callgrind can give answers that are repeatable to 7 or more significant
  digits. In comparison, actual (wall-clock) run times are scarcely repeatable
  beyond one significant digit.

  This allows to implement and measure "microoptimizations". Typical
  microoptimizations reduce the number of CPU cycles by `0.1%` or `0.05%` or
  even less. Such improvements are impossible to measure with real-world
  timings. But hundreds or thousands of microoptimizations add up, resulting in
  measurable real-world performance gains.[^note]
* Iai-Callgrind can work reliably in noisy environments especially in CI
  environments from providers like GitHub Actions or Travis-CI, where
  Criterion-rs cannot.
* The benchmark api of Iai-Callgrind is simple, intuitive and allows for a much
  more concise and clearer structure of benchmarks.
* Iai-Callgrind can benchmark functions in binary crates.
* Iai-Callgrind can benchmark private functions.
* Although Callgrind adds runtime overhead, running each benchmark exactly once
  is still usually much faster than Criterion-rs' statistical measurements.
* Criterion-rs creates plots and graphs about the averages, median etc. which
  adds considerable execution time to the execution time for each benchmark.
  Iai-Callgrind doesn't need any of these plots, since it can collect all its
  metrics in a single run.
* Iai-Callgrind generates profile output from the benchmark without further
  effort.
* With Iai-Callgrind you have native access to all the possibilities of all
  Valgrind tools, including Valgrind Client Requests.

Iai-Callgrind/Criterion-rs Mixed:

* Although it is usually not significant, due to the high precision of the
  Iai-Callgrind measurements changes in the benchmarks themselves like adding a   
  benchmark case can have an effect on the other benchmarks. Iai-Callgrind can
  only try to reduce these effects to a minimum but never completely eliminate
  them. Criterion-rs does not have this problem because it cannot detect such
  small changes.

Iai-Callgrind Cons:

* Iai-Callgrind's measurements merely correlate with wall-clock time. Wall-clock
  time is an obvious choice in many cases because it corresponds to what users
  perceive and Criterion-rs measures it directly.
* Iai-Callgrind can only be used on platforms supported by Valgrind. Notably,
  this does not include Windows.
* Iai-Callgrind needs additional binaries, `valgrind` and the
  `iai-callgrind-runner`. The version of the runner needs to be in sync with the
  `iai-callgrind` library. Criterion-rs is only a library and the installation
  is usually simpler.

Especially, due to the first point in the `Cons`, I think it is still required
to run wall-clock time benchmarks and use `Criterion-rs` in conjunction with
Iai-Callgrind. But in the CI and for performance regression checks, you
shouldn't use `Criterion-rs` or other wall-clock time based benchmarks at all.

[^note]: <https://sqlite.org/cpu.html#performance_measurement>
