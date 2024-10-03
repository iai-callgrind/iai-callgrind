# Vision

This is a collection of long-term ideas for Iai-Callgrind.

If you have further ideas, please start a discussion or issue.

| Idea | Further Description |
| ---- | ----------- |
| Html reports| Create html reports with integrated callgrind flamegraphs. The flamegraphs would not only show the current and the old run but also the difference between the two runs. The latter is not possible with native callgrind tools. The html reports would be a modern version of the `callgrind_annotate` output. The html reports should also contain the source code and the related metrics. |
| Ad-hoc benchmarking | Benchmark a binary or maybe even a library function (the latter might not be feasible) from the command-line for ad-hoc profiling. For example `iai-callgrind-runner ad-hoc --tools dhat target/release/my-binary --args --to --binary` would benchmark `my-binary` with the specified arguments with the Iai-Callgrind framework without having to create a benchmark in a benchmark file. |
| Low-level api for library benchmarks | Sometimes it would be great to have a low-level api for library benchmarks much like the low-level api for binary benchmarks. |
| Show the execution times of each benchmark | This is not meant to be a benchmark metric. It would simply show how long the valgrind execution took to run. Valgrind adds some overhead to the execution time of the benchmarked function/binary, and the execution time would help to understand how much this overhead actually is and if it is a concern. |
| Option or feature gate to run cachegrind instead of callgrind | Using `cachegrind` instead of `callgrind` is meant to be a fallback if something's not working with `callgrind`. |
| Improve DHAT heap usage profiles | DHAT creates the heap usage profile in library benchmarks with the heap usage of the `setup` and `teardown` functions and any code before and after the benchmark function. Iai-Callgrind should be able to apply filters on the dhat heap usage profiles much like callgrind toggles: `some_group::benchmark_function::id` would only show the heap usage of the benchmark function. `my_lib::some_func` would only show the heap usage of the function `my_lib::some` and so on. This is just an idea and hasn't been checked to be actually feasible. |
| Run the `vgdb` debugger for a specific benchmark on demand | This could be made possible with a command-line argument `--vgdb benchmark::some_group::function_name::id` or environment variable. In such a case no benchmarks are run besides the specified one and the debugger is started automatically. This enables for example the usage of valgrind monitor commands. |
| Optionally run benchmarks through docker | Optionally running the Iai-Callgrind benchmarks with docker (via an environment variable, command-line argument) would make it possible to painlessly run the benchmarks on a windows or macos host or for different targets. The docker images might be based on the `cross` images with valgrind pre-installed. But maybe it's necessary or easier to provide own docker images. |
