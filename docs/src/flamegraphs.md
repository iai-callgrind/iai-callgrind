# Callgrind Flamegraphs

Flamegraphs are opt-in and can be created if you pass a `FlamegraphConfig` to
the `BinaryBenchmarkConfig` or `LibraryBenchmarkConfig`. Callgrind flamegraphs
are meant as a complement to valgrind's visualization tools
`callgrind_annotate` and `kcachegrind`.

For example create differential flamegraphs for all benchmarks in a library
benchmark:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    FlamegraphConfig, FlamegraphKind
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
        .flamegraph(FlamegraphConfig::default()
            .kind(FlamegraphKind::Differential)
        );
    library_benchmark_groups = my_group
);
# }
```

Callgrind flamegraphs show the inclusive costs for functions and a single
`EventKind` (default is `EventKind::Ir`), similar to `callgrind_annotate` but in
a nicer (and clickable) way. Especially, differential flamegraphs facilitate a
deeper understanding of code sections which cause a bottleneck or a performance
regressions etc.

The produced flamegraph `*.svg` files are located next to the respective
callgrind output file in the `target/iai` directory.
