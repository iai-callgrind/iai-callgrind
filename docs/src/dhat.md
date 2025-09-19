<!-- markdownlint-disable MD041 MD033 -->

# DHAT: a dynamic heap analysis tool

## Intro to DHAT

To fully understand DHAT please read the [Valgrind docs][Dhat] of DHAT. Here's
just a short summary and quote from the docs:

> DHAT is primarily a tool for examining how programs use their heap
> allocations. It tracks the allocated blocks, and inspects every memory access
> to find which block, if any, it is to. It presents, on a program point basis,
> information about these blocks such as sizes, lifetimes, numbers of reads and
> writes, and read and write patterns.

The rest of this chapter is dedicated to how DHAT is integrated into
Gungraun.

## The DHAT modes

Gungraun supports all three modes `heap` (the default), `copy` and `ad-hoc`
which can be changed on the [command-line](./cli_and_env/basics.md) with
`--dhat-args=--mode=ad-hoc` or in the benchmark itself with `Dhat::args`. Note
that `ad-hoc` mode requires [client requests](./client_requests.md) which have
prerequisites. If running the benchmarks in `ad-hoc` mode, it is highly
recommended to turn off the `EntryPoint` with `EntryPoint::None` (See next
section). However, DHAT is normally run in `heap` mode and it is assumed that
this is the mode used in the next sections.

## The default entry point

The DHAT default entry point `EntryPoint::Default` in library benchmarks behaves
similar to [`Callgrind's
EntryPoint`](./benchmarks/library_benchmarks/custom_entry_point.md). This
centers the collected metrics shown in the terminal output around the benchmark
function. The entry point is set to `EntryPoint::None` for binary benchmarks.
But, if necessary, the entry point can be turned off or customized in
`Dhat::entry_point`.

In contrast to callgrind's entry point, the DHAT default entry point *includes*
the metrics of [`setup` and/or `teardown`
code](./benchmarks/library_benchmarks/setup_and_teardown.md) or anything
specified in the `args` parameter of the `#[bench]` or `#[benches]` attribute.
This is a limitation of DHAT and what is possible to reliably extract from the
output files. Callgrind has a command-line flag `--toggle-collect` to toggle
collection on and off. DHAT doesn't have such an option, and the sanitization of
metrics can only be realized afterwards based on the DHAT output files. However,
this works well enough to stabilize the metrics so they exclude the metrics of
Gungraun allocations (around 2000 - 2500 bytes) in the `main` function
needed to setup the benchmark.

Note that setting an entry point or `Dhat::frames` does not alter the dhat
output files in any way.

## Usage on the command-line

Running DHAT instead of or in addition to Callgrind is pretty straight-forward
and not different to any [other tool](./tools.md):

Either use [command-line arguments or environment
variables](./cli_and_env/basics.md): `--default-tool=dhat` or
`IAI_CALLGRIND_DEFAULT_TOOL=dhat` (replaces callgrind as default tool) or
`--tools=dhat` or `IAI_CALLGRIND_TOOLS=dhat` (runs DHAT in addition to the
default tool).

## Usage in a benchmark and a small example analysis

Running DHAT in addition to Callgrind can also be carried out in the benchmark
itself with the `Dhat` struct in `LibraryBenchmarkConfig::tool`. Here, globally
in the `main!` macro:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    Dhat
};
use std::hint::black_box;

#[library_benchmark]
#[bench::worst_case_3(vec![3, 2, 1])]
fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_bubble_sort);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default());
    library_benchmark_groups = my_group
);
# }
```

The above benchmark will produce the following metrics:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_dhat::my_group::bench_library</span> <span style="color:#0AA">worst_case_3</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [3, 2, 1]</span></b>
<span style="color:#555">  </span><span style="color:#555">=======</span> CALLGRIND <span style="color:#555">====================================================================</span>
<span style="color:#555">  </span>Instructions:                          <b>83</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                              <b>110</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>LL Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>3</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                     <b>113</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                     <b>215</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#555">=======</span> DHAT <span style="color:#555">=========================================================================</span>
<span style="color:#555">  </span>Total bytes:                           <b>12</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total blocks:                           <b>1</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax bytes:                        <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax blocks:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end bytes:                         <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end blocks:                        <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Reads bytes:                           <b>24</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Writes bytes:                          <b>36</b>|N/A                  (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.55554s</code></pre>

Analyzing the DHAT data, there are a total of `12 bytes` of allocations (The
vector: `3 * sizeof(i32)` bytes = `3 * 4` bytes) in `1` block during the setup
of the benchmark. That's also `12` bytes of writes to fill the vector with the
values. That makes `24` bytes of reads and `24` bytes of writes in the
`bubble_sort` function. Also, there are no (de-)allocations of heap memory in
`bubble_sort` itself.

## Soft limits and hard limits

Based on that data, we could define for example hard limits (or soft limits or
both whatever you think is appropriate) to ensure `bubble_sort` is not getting
worse than that.

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    Dhat, DhatMetric
};
use std::hint::black_box;

#[library_benchmark]
#[bench::worst_case_3(
    args = (vec![3, 2, 1]),
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default()
            .hard_limits([
                (DhatMetric::ReadsBytes, 24),
                (DhatMetric::WritesBytes, 32)
            ])
        )
)]
fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_bubble_sort);

# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Dhat::default());
    library_benchmark_groups = my_group
);
# }
```

Now, if `bubble_sort` would read more than `24` bytes or if there were more than
`32` bytes of writes during the benchmark, the benchmark would fail and exit
with error.

## Frames and benchmarking multi-threaded functions

It is possible to specify additional `Dhat::frames` for example when
benchmarking multi-threaded functions. Like in callgrind, each thread/subprocess
in DHAT is treated as a separate unit and thus requires `frames` (the
Gungraun specific approximation of callgrind toggles) in addition to the
default entry point to include the interesting ones in the measurements.

By example. Suppose there's a function in the `benchmark_tests` library
`find_primes_multi_thread(num_threads: usize)` which searches for primes in the
range `0` - `10000 * num_threads`. This multi-threaded function is splitting the
work for each `10000` numbers into a separate thread each calling the
single-threaded function `benchmark_tests::find_primes` which does the actual
work. The inner workings aren't important but this description should be enough
to understand the basic idea.

```rust
# extern crate iai_callgrind;
# mod benchmark_tests { pub fn find_primes_multi_thread (_: u64) -> Vec<u64> { vec![] } }
use std::hint::black_box;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    ValgrindTool,
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DHAT)
)]
fn bench_library() -> Vec<u64> {
    black_box(benchmark_tests::find_primes_multi_thread(black_box(1)))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Running the benchmark produces the following output:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_find_primes::my_group::bench_library</span>
<span style="color:#555">  </span><span style="color:#555">=======</span> DHAT <span style="color:#555">=========================================================================</span>
<span style="color:#555">  </span>Total bytes:                        <b>11456</b>|11456                (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Total blocks:                           <b>9</b>|9                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>At t-gmax bytes:                    <b>10264</b>|10264                (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>At t-gmax blocks:                       <b>4</b>|4                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>At t-end bytes:                         <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>At t-end blocks:                        <b>0</b>|0                    (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Reads bytes:                          <b>776</b>|776                  (<span style="color:#555">No change</span>)
<span style="color:#555">  </span>Writes bytes:                       <b>10329</b>|10329                (<span style="color:#555">No change</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.44534s</code></pre>

The problem here is, that the spawned thread is not included in the metrics.
Looking at the output files of the dhat output in `dh_view.html` (heavily
shortened to safe some space):

```text
Invocation {
  Mode:    heap
  Command: /home/some/project/target/release/deps/lib_bench_find_primes-c304b7c3fed25785 --iai-run my_group 0 0 lib_bench_find_primes::my_group::bench_library
  PID:     212817
}

Times {
  t-gmax: 2,825,042 instrs (99.57% of program duration)
  t-end:  2,837,309 instrs
}

▼ PP 1/1 (3 children) {
    Total:     46,827 bytes (100%, 16,504.02/Minstr) in 37 blocks (100%, 13.04/Minstr), avg size 1,265.59 bytes, avg lifetime 840,789.86 instrs (29.63% of program duration)
    At t-gmax: 26,847 bytes (100%) in 9 blocks (100%), avg size 2,983 bytes
    At t-end:  0 bytes (0%) in 0 blocks (0%), avg size 0 bytes
    Reads:     45,876 bytes (100%, 16,168.84/Minstr), 0.98/byte
    Writes:    48,285 bytes (100%, 17,017.89/Minstr), 1.03/byte
    Allocated at {
      #0: [root]
    }
  }
  ├─▼ PP 1.1/3 (12 children) {
  │     Total:     46,027 bytes (98.29%, 16,222.06/Minstr) in 28 blocks (75.68%, 9.87/Minstr), avg size 1,643.82 bytes, avg lifetime 858,562.71 instrs (30.26% of program duration)
  │     At t-gmax: 26,511 bytes (98.75%) in 7 blocks (77.78%), avg size 3,787.29 bytes
  │     At t-end:  0 bytes (0%) in 0 blocks (0%), avg size 0 bytes
  │     Reads:     45,412 bytes (98.99%, 16,005.31/Minstr), 0.99/byte
  │     Writes:    47,925 bytes (99.25%, 16,891/Minstr), 1.04/byte
  │     Allocated at {
  │       #1: 0x48C57A8: malloc (in /usr/lib/valgrind/vgpreload_dhat-amd64-linux.so)
  │     }
  │   }
  │   ├── PP 1.1.1/12 {
  │   │     Total:     32,736 bytes (69.91%, 11,537.69/Minstr) in 10 blocks (27.03%, 3.52/Minstr), avg size 3,273.6 bytes, avg lifetime 235,111.9 instrs (8.29% of program duration)
  │   │     Max:       16,384 bytes in 1 blocks, avg size 16,384 bytes
  │   │     At t-gmax: 16,384 bytes (61.03%) in 1 blocks (11.11%), avg size 16,384 bytes
  │   │     At t-end:  0 bytes (0%) in 0 blocks (0%), avg size 0 bytes
  │   │     Reads:     26,184 bytes (57.08%, 9,228.46/Minstr), 0.8/byte
  │   │     Writes:    26,184 bytes (54.23%, 9,228.46/Minstr), 0.8/byte
  │   │     Allocated at {
  │   │       ^1: 0x48C57A8: malloc (in /usr/lib/valgrind/vgpreload_dhat-amd64-linux.so)
  │   │       #2: 0x40153B7: UnknownInlinedFun (alloc.rs:93)
  │   │       #3: 0x40153B7: UnknownInlinedFun (alloc.rs:188)
  │   │       #4: 0x40153B7: UnknownInlinedFun (alloc.rs:249)
  │   │       #5: 0x40153B7: UnknownInlinedFun (mod.rs:476)
  │   │       #6: 0x40153B7: with_capacity_in<alloc::alloc::Global> (mod.rs:422)
  │   │       #7: 0x40153B7: with_capacity_in<u64, alloc::alloc::Global> (mod.rs:190)
  │   │       #8: 0x40153B7: with_capacity_in<u64, alloc::alloc::Global> (mod.rs:815)
  │   │       #9: 0x40153B7: with_capacity<u64> (mod.rs:495)
  │   │       #10: 0x40153B7: from_iter<u64, core::iter::adapters::filter::Filter<core::ops::range::RangeInclusive<u64>, benchmark_tests::find_primes::{closure_env#0}>> (spec_from_iter_nested.rs:31)
  │   │       #11: 0x40153B7: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter::SpecFromIter<T,I>>::from_iter (spec_from_iter.rs:34)
  │   │       #12: 0x4013B67: from_iter<u64, core::iter::adapters::filter::Filter<core::ops::range::RangeInclusive<u64>, benchmark_tests::find_primes::{closure_env#0}>> (mod.rs:3438)
  │   │       #13: 0x4013B67: collect<core::iter::adapters::filter::Filter<core::ops::range::RangeInclusive<u64>, benchmark_tests::find_primes::{closure_env#0}>, alloc::vec::Vec<u64, alloc::alloc::Global>> (iterator.rs:2001)
  │   │       #14: 0x4013B67: benchmark_tests::find_primes (lib.rs:25)
  │   │       #15: 0x4015800: {closure#0} (lib.rs:32)
  │   │       #16: 0x4015800: std::sys::backtrace::__rust_begin_short_backtrace (backtrace.rs:152)
  │   │       #17: 0x4014824: {closure#0}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>> (mod.rs:559)
  │   │       #18: 0x4014824: call_once<alloc::vec::Vec<u64, alloc::alloc::Global>, std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>>> (unwind_safe.rs:272)
  │   │       #19: 0x4014824: do_call<core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>>>, alloc::vec::Vec<u64, alloc::alloc::Global>> (panicking.rs:589)
  │   │       #20: 0x4014824: try<alloc::vec::Vec<u64, alloc::alloc::Global>, core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>>>> (panicking.rs:552)
  │   │       #21: 0x4014824: catch_unwind<core::panic::unwind_safe::AssertUnwindSafe<std::thread::{impl#0}::spawn_unchecked_::{closure#1}::{closure_env#0}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>>>, alloc::vec::Vec<u64, alloc::alloc::Global>> (panic.rs:359)
  │   │       #22: 0x4014824: {closure#1}<benchmark_tests::find_primes_multi_thread::{closure_env#0}, alloc::vec::Vec<u64, alloc::alloc::Global>> (mod.rs:557)
  │   │       #23: 0x4014824: core::ops::function::FnOnce::call_once{{vtable.shim}} (function.rs:250)
  │   │       #24: 0x404460A: call_once<(), dyn core::ops::function::FnOnce<(), Output=()>, alloc::alloc::Global> (boxed.rs:1966)
  │   │       #25: 0x404460A: call_once<(), alloc::boxed::Box<dyn core::ops::function::FnOnce<(), Output=()>, alloc::alloc::Global>, alloc::alloc::Global> (boxed.rs:1966)
  │   │       #26: 0x404460A: std::sys::pal::unix::thread::Thread::new::thread_start (thread.rs:97)
  │   │       #27: 0x49BB7EA: ??? (in /usr/lib/libc.so.6)
  │   │       #28: 0x4A3EFB3: clone (in /usr/lib/libc.so.6)
  │   │     }
  │   │   }
  ...
```

The missing metrics of the thread are caused by the default entry point which
only includes the program points with the benchmark function in their call
stack. But, looking closely at the program point `PP 1.1.1/12` and the call
stack, there's no frame (function call) of the benchmark function
`bench_library` or a `main` function. As mentioned earlier, this is because the
thread is completely separated by DHAT.

There are multiple ways to go on depending on what we want to measure. To show
two different approaches, at first, I'll go with measuring the benchmark
function with the function spawning the threads (the default entry point which
doesn't have to be specified) and additionally all threads which execute the
`benchmark_tests::find_primes` function.

```rust
# extern crate iai_callgrind;
# mod benchmark_tests { pub fn find_primes_multi_thread (_: u64) -> Vec<u64> { vec![] } }
use std::hint::black_box;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    ValgrindTool, Dhat
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DHAT)
        .tool(Dhat::default()
            .frames(["benchmark_tests::find_primes"])
        )
)]
fn bench_library() -> Vec<u64> {
    black_box(benchmark_tests::find_primes_multi_thread(black_box(1)))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Now, the metrics include the spawned thread(s):

<pre><code class="hljs"><span style="color:#0A0">lib_bench_find_primes::my_group::bench_library</span>
<span style="color:#555">  </span><span style="color:#555">=======</span> DHAT <span style="color:#555">=========================================================================</span>
<span style="color:#555">  </span>Total bytes:                        <b>44192</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total blocks:                          <b>19</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax bytes:                    <b>26648</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax blocks:                       <b>5</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end bytes:                         <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end blocks:                        <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Reads bytes:                        <b>26960</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Writes bytes:                       <b>36513</b>|N/A                  (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.44273s</code></pre>

If we were only interested in the threads itself, then using
`EntryPoint::Custom` would be one way to do it. Setting a custom entry point is
sugar for disabling the entry point with `EntryPoint::None` and specifying a
frame with `Dhat::frames`:

```rust
# extern crate iai_callgrind;
# mod benchmark_tests { pub fn find_primes_multi_thread (_: u64) -> Vec<u64> { vec![] } }
use std::hint::black_box;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    ValgrindTool, Dhat, EntryPoint
};

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .default_tool(ValgrindTool::DHAT)
        .tool(Dhat::default()
            .entry_point(
                EntryPoint::Custom("benchmark_tests::find_primes".to_owned())
            )
        )
)]
fn bench_library() -> Vec<u64> {
    black_box(benchmark_tests::find_primes_multi_thread(black_box(1)))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Running this benchmark results in:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_find_primes::my_group::bench_library</span>
<span style="color:#555">  </span><span style="color:#555">=======</span> DHAT <span style="color:#555">=========================================================================</span>
<span style="color:#555">  </span>Total bytes:                        <b>32736</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total blocks:                          <b>10</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax bytes:                    <b>16384</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-gmax blocks:                       <b>1</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end bytes:                         <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>At t-end blocks:                        <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Reads bytes:                        <b>26184</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Writes bytes:                       <b>26184</b>|N/A                  (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.45178s</code></pre>

To verify our setup, let's compare these numbers with the data of the program
point with the thread of the `dh_view.html` output shown above. Eventually,
these are the same metrics:

```text
  │   ├── PP 1.1.1/12 {
  │   │     Total:     32,736 bytes (69.91%, 11,537.69/Minstr) in 10 blocks (27.03%, 3.52/Minstr), avg size 3,273.6 bytes, avg lifetime 235,111.9 instrs (8.29% of program duration)
  │   │     Max:       16,384 bytes in 1 blocks, avg size 16,384 bytes
  │   │     At t-gmax: 16,384 bytes (61.03%) in 1 blocks (11.11%), avg size 16,384 bytes
  │   │     At t-end:  0 bytes (0%) in 0 blocks (0%), avg size 0 bytes
  │   │     Reads:     26,184 bytes (57.08%, 9,228.46/Minstr), 0.8/byte
  │   │     Writes:    26,184 bytes (54.23%, 9,228.46/Minstr), 0.8/byte
```

[Dhat]: https://valgrind.org/docs/manual/dh-manual.html
