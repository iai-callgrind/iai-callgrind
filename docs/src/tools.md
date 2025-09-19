# Other Valgrind Tools

In addition to or instead of the default tool `Callgrind`, you can use the
Gungraun framework to run other Valgrind profiling tools like
[`DHAT`](./dhat.md), `Massif` or even [`Cachegrind`](./cachegrind.md) and the
experimental `BBV` but also error checking tools like `Memcheck`, `Helgrind` and
`DRD`.

Note that support for `Massif` or `BBV` is currently only basic and doesn't show
useful stats and metrics in the terminal output of Gungraun. But, the
output files are generated as usual and are ready to be examined with tools like
`ms_print`.

See also the [Valgrind User
Manual](https://valgrind.org/docs/manual/manual.html) for all the details about
each tool and their command line arguments.

## Running other Valgrind tools

It's possible to change the default tool `Callgrind` to any other valgrind tool
with the [command-line argument](./cli_and_env/basics.md)
`--default-tool=<tool>` or environment variable
`IAI_CALLGRIND_DEFAULT_TOOL=<tool>`. `<tool>` may be one of `callgrind`,
`cachegrind`, `dhat`, `massif`, `memcheck`, `helgrind`, `drd`, `exp-bbv`.

Running tools in addition to the default tool can be achieved with
`--tools=<tools>` or `IAI_CALLGRIND_TOOLS=<tools>` where `<tools>` is a
`,`-separated list of one or more of the `<tool>` above.

The tool configurations can be changed in the benchmark file by specifying the
structs `Callgrind`, `Cachegrind`, ..., `Bbv` in `LibraryBenchmarkConfig::tool`
or `BinaryBenchmarkConfig::tool`.

Note that it is fully sufficient to specify a configuration to actually run the
tool. For example to run [`DHAT`](./dhat.md) with its default configuration for
all library benchmarks in the same file in addition to `Callgrind` without the
need for `--tools=dhat`:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    ValgrindTool, Dhat
};
use std::hint::black_box;

#[library_benchmark]
fn bench_library() -> Vec<i32> {
    black_box(my_lib::bubble_sort(vec![3, 2, 1]))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default());
    library_benchmark_groups = my_group
);
# }
```

All tools which produce an `ERROR SUMMARY` `(Memcheck, DRD, Helgrind)` have
[`--error-exitcode=201`](https://valgrind.org/docs/manual/manual-core.html#manual-core.erropts)
set, so if there are any errors, the benchmark run fails with `201`. You can
overwrite this default for example for `Memcheck`

```rust
# extern crate iai_callgrind;
use iai_callgrind::{Memcheck, ValgrindTool};

Memcheck::with_args(["--error-exitcode=0"]);
```

which would restore the default of `0` from valgrind.
