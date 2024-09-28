# Overview

Iai-Callgrind can be used to benchmark the [library](./library_benchmarks.md)
and [binary](./binary_benchmarks.md) of your project's crates. Library and
binary benchmarks are treated differently by Iai-Callgrind and cannot be
intermixed in the same benchmark file. This is indeed a feature and helps
keeping things organized. Having different and multiple benchmark files for
library and binary benchmarks is no problem for Iai-Callgrind and is usually a
good idea anyway. Having benchmarks for different binaries in the same
benchmark file however is fully supported.

Head over to the [Quickstart](./library_benchmarks/quickstart.md) section of
library benchmarks if you want to start benchmarking your library functions or
to the [Quickstart](./binary_benchmarks/quickstart.md) section of binary
benchmarks if you want to start benchmarking your crate's binary (binaries).

## Binary Benchmarks vs Library Benchmarks

Almost all binary benchmarks can be written as library benchmarks. For example,
if you have a `main.rs` file of your binary, which basically looks like this

```rust
# mod my_lib { pub fn run() {} }
use my_lib::run;

fn main() {
    run();
}
```

you could also choose to benchmark the library function `my_lib::run` in a
library benchmark instead of the binary in a binary benchmark. There's no real
downside to either of the benchmark schemes and which scheme you want to use
heavily depends on the structure of your binary. As a maybe obvious rule of
thumb, micro-benchmarks of specific functions should go into library benchmarks
and macro-benchmarks into binary benchmarks. Generally, choose the closest
access point to the program point you actually want to benchmark.

You should always choose binary benchmarks over library benchmarks if you want
to benchmark the behaviour of the executable if the input comes from a pipe
since this feature is exclusive to binary benchmarks. See [The Command's stdin
and simulating piped input](./binary_benchmarks/stdin_and_pipe.md) for more.
