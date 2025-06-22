# Cachegrind

## Prerequisites

In order to use `Cachegrind` instead of `Callgrind` you need valgrind version
`3.22` or above installed (which you can look up with `valgrind --version`). In
this version Valgrind introduced the two [Client requests](./client_requests.md)
`start_instrumentation()` and `stop_instrumentation()`. In order to use client
requests you need to turn them on in the `Cargo.toml` with the `client_requests`
feature

```toml
[dev-dependencies]
iai-callgrind = { version = "0.15.0", features = ["client_requests"] }
```

## The cachegrind feature

There are two ways to use cachegrind instead of callgrind. The first and easy
way is to use the `cachegrind` feature, so your `iai-callgrind` spec should
finally look like this:

```toml
[dev-dependencies]
iai-callgrind = { version = "0.15.0", features = ["cachegrind"] }
```

The `cachegrind` feature automatically activates the `client_requests` feature,
and there's no need to specify it again. Now, without having to do anything
else, all benchmarks run with Cachegrind instead of Callgrind. However, this
change has implications which are better understood by showing the second way.

## The second way

There are actually multiple second ways to run Cachegrind as default tool (see
also command-line arguments) but they have the same principle in common. For
example in the benchmark file run a specific benchmark function with Cachegrind:

```rust
# extern crate iai_callgrind;
# pub mod my_lib { pub fn bubble_sort(input: Vec<i32>) -> Vec<i32> { input } }

use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    client_requests, ValgrindTool
};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Cachegrind)
)]
#[bench::small(vec![3, 2, 1])]
#[bench::bigger(vec![5, 4, 3, 2, 1])]
fn bench_function(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_function);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

However, this is not enough to get correct measurements. Only choosing
`Cachegrind` as default tool will measure everything including setup and
teardown, ... For this reason we need client requests to tell `Cachegrind` when
to start and stop the instrumentation:

```rust
# extern crate iai_callgrind;
# pub mod my_lib { pub fn bubble_sort(input: Vec<i32>) -> Vec<i32> { input } }

use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    ValgrindTool, client_requests, Cachegrind
};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::Cachegrind)
        .tool(Cachegrind::with_args(["--instr-at-start=no"]))
)]
#[bench::small(vec![3, 2, 1])]
#[bench::bigger(vec![5, 4, 3, 2, 1])]
fn bench_function(array: Vec<i32>) -> Vec<i32> {
    client_requests::cachegrind::start_instrumentation();
    let r = black_box(my_lib::bubble_sort(array));
    client_requests::cachegrind::stop_instrumentation();
    r
}

library_benchmark_group!(name = my_group; benchmarks = bench_function);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Not only the body of the benchmark function changed but also the command-line
argument `--instr-at-start=no` had to be specified in order to start the
instrumentation with the client request and not (what is the default) when
starting the benchmark executable.

All of the above is exactly what the `cachegrind` feature does. It adds the
client requests to the function body, returns the result from the function and
start cachegrind with `--instr-at-start=no`. The consequence and a disadvantage
of cachegrind is that the function body had to be altered a little bit. It's not
much but running other tools tools like `Callgrind` on the same benchmark
function like `Cachegrind` would show small differences because the client
requests add `10` - `20` instructions to the function body.

## When to use Cachegrind

As shown above, running `Cachegrind` can have disadvantages but there are
circumstances under which it is better to use `Cachegrind`. Here's a comparison
of both tools:

| Cachegrind | Callgrind |
| -- | -- |
| Works on all platforms | Callgrind's ability to detect function calls and returns depends on the instruction set of the platform it is run on. It works best on x86 and amd64, and unfortunately currently does not work so well on PowerPC, ARM, Thumb or MIPS code. This is because there are no explicit call or return instructions in these instruction sets, so Callgrind has to rely on heuristics to detect calls and returns |
| Bigger tool set: `cg_diff`, `cg_merge` and `cg_annotate` | Just `callgrind_annotate` |
| Smaller functionality which shows in a far less amount of command-line arguments | Greater functionality (`--toggle-collect`, ...) |
| Smaller amount of profile data and metrics | More metrics (`--collect-bus`, ...) |
| Client requests add a small amount of build time and have more prerequisites | No need for client requests and no alteration of the benchmark function body is required which makes it more intuitive to use |
