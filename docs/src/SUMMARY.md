<!-- markdownlint-disable MD025 MD042 -->
# Summary

- [Introduction](./intro.md)
- [Getting Help](./getting_help.md)

# Installation

- [Prerequisites](./installation/prerequisites.md)
- [Iai-Callgrind](./installation/iai_callgrind.md)

# Benchmarks

- [Overview](./benchmarks/overview.md)
- [Library Benchmarks](./benchmarks/library_benchmarks.md)
    - [Important default behaviour](./benchmarks/library_benchmarks/important.md)
    - [Quickstart](./benchmarks/library_benchmarks/quickstart.md)
    - [Anatomy of a library benchmark](./benchmarks/library_benchmarks/anatomy.md)
    - [The macros in more detail](./benchmarks/library_benchmarks/macros.md)
    - [setup and teardown](./benchmarks/library_benchmarks/setup_and_teardown.md)
    - [Specify multiple benches at once](./benchmarks/library_benchmarks/multiple_benches.md)
    - [Generic benchmark functions](./benchmarks/library_benchmarks/generic.md)
    - [Comparing benchmark functions](./benchmarks/library_benchmarks/compare_by_id.md)
    - [Configuration](./benchmarks/library_benchmarks/configuration.md)
        - [Output Format/Cache Misses](./benchmarks/library_benchmarks/configuration/output_format.md)
    - [Custom entry points](./benchmarks/library_benchmarks/custom_entry_point.md)
    - [Multi-threaded and multi-process applications](./benchmarks/library_benchmarks/threads_and_subprocesses.md)
    - [More Examples, please!](./benchmarks/library_benchmarks/examples.md)
- [Binary Benchmarks](./benchmarks/binary_benchmarks.md)
    - [Important default behaviour](./benchmarks/binary_benchmarks/important.md)
    - [Quickstart](./benchmarks/binary_benchmarks/quickstart.md)
    - [Differences to library benchmarks](./benchmarks/binary_benchmarks/differences.md)
    - [The Command's stdin and simulating piped input](./benchmarks/binary_benchmarks/stdin_and_pipe.md)
    - [Configuration](./benchmarks/binary_benchmarks/configuration.md)
        - [Delay the Command](./benchmarks/binary_benchmarks/configuration/delay.md)
        - [Sandbox](./benchmarks/binary_benchmarks/configuration/sandbox.md)
        - [Configure the exit code of the Command](./benchmarks/binary_benchmarks/configuration/exit_code.md)
    - [Low-level api](./benchmarks/binary_benchmarks/low_level.md)
    - [More examples needed?](./benchmarks/binary_benchmarks/examples.md)

- [Performance Regressions](./regressions.md)
- [Other Valgrind Tools](./tools.md)
- [Valgrind Client Requests](./client_requests.md)
- [Callgrind Flamegraphs](./flamegraphs.md)

# Command-line and environment variables

- [Basic usage and exit codes](./cli_and_env/basics.md)
- [Comparing with baselines](./cli_and_env/baselines.md)
- [Controlling the output of Iai-Callgrind](./cli_and_env/output.md)
    - [Customize the output directory](./cli_and_env/output/out_directory.md)
    - [Machine-readable output](./cli_and_env/output/machine_readable.md)
    - [Showing terminal output of benchmarks](./cli_and_env/output/terminal_output.md)
    - [Changing the color output](./cli_and_env/output/color.md)
    - [Changing the logging output](./cli_and_env/output/logging.md)

# Troubleshooting

- [I'm getting the error `Sentinel ... not found`](./troubleshooting/im-getting-the-error-sentinel-not-found.md)
- [Running `cargo bench` results in an "Unrecognized Option" error](./troubleshooting/running-cargo-bench-results-in-an-unrecognized-option-error.md)

# Comparison

- [Criterion](./comparison/criterion.md)
- [Iai](./comparison/iai.md)
