# Other Valgrind Tools

In addition to the default benchmarks, you can use the Iai-Callgrind framework
to run other Valgrind profiling `Tool`s like `DHAT`, `Massif` and the
experimental `BBV` but also `Memcheck`, `Helgrind` and `DRD` if you need to
check memory and thread safety of benchmarked code. See also the [Valgrind User
Manual](https://valgrind.org/docs/manual/manual.html) for more details and
command line arguments. The additional tools can be specified in a
`LibraryBenchmarkConfig` or `BinaryBenchmarkConfig`. For example to run `DHAT`
for all library benchmarks in addition to `Callgrind`:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig, 
    Tool, ValgrindTool
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
        .tool(Tool::new(ValgrindTool::DHAT));
    library_benchmark_groups = my_group
);
# }
```

All tools which produce an `ERROR SUMMARY` `(Memcheck, DRD, Helgrind)` have
[`--error-exitcode=201`](https://valgrind.org/docs/manual/manual-core.html#manual-core.erropts)
set, so if there are any errors, the benchmark run fails with `201`. You can
overwrite this default with

```rust
# extern crate iai_callgrind;
use iai_callgrind::{Tool, ValgrindTool};

Tool::new(ValgrindTool::Memcheck).args(["--error-exitcode=0"]);
```

which would restore the default of `0` from valgrind.