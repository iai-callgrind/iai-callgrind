<!-- markdownlint-disable MD025 MD042 -->
# Summary

- [Introduction](./intro.md)
- [Getting Help](./getting_help.md)

# Installation

- [Prerequisites](./installation/prerequisites.md)
- [Iai-Callgrind](./installation/iai_callgrind.md)

# Benchmarks

- [Library Benchmarks]()
    - [Important default behaviour]()
    - [Quickstart]()
    - [The `#[library_benchmark]` attribute]()
    - [The `#[bench]` attribute]()
    - [The `#[benches]` attribute]()
    - [The `library_benchmark_group!` macro]()
    - [The `main!` macro]()
    - [Generic benchmark functions]()
    - [Comparing benchmark functions]()
    - [Configuration]()
    - [Examples]()
- [Binary Benchmarks]()
    - [Important default behaviour]()
    - [Quickstart]()
    - [Differences to library benchmarks]()
        - [setup and teardown]()
    - [The Sandbox]()
    - [The Command's stdin and simulating piped input]()
    - [Configure the exit code of the Command]()
    - [Low level api]()
        - [Intermixing high-level and low-level api]()
        - [The binary_benchmark_attribute! macro]()
    - [Examples]()

- [Performance Regressions]()
- [Valgrind Tools]()
- [Valgrind Client Requests]()
- [Flamegraphs]()

# Command-line and environment variables

- [Basic usage]()
- [Comparing with baselines]()
- [Controlling the output of iai-callgrind]()
    - [Customize the output directory]()
    - [Machine-readable output]()
    - [Showing terminal output of benchmarks](./cli_and_env/output/terminal_output.md)
    - [Changing the color output]()
    - [Changing the logging output]()

# Examples

- [Server]()

# Troubleshooting

- [I'm getting the error `Sentinel ... not found`]()
- [Running `cargo bench` results in an "Unrecognized Option" error]()

# Comparison

- [Criterion]()
- [Iai]()

# Upgrading

# [0.12.3 -> 0.13.0]()
