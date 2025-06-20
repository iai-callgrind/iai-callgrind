<!-- markdownlint-disable MD025 MD042 MD033 -->

# Multi-threaded and multi-process applications

The default is to run Iai-Callgrind benchmarks with `--separate-threads=yes`,
`--trace-children=yes` switched on. This enables Iai-Callgrind to trace threads
and subprocesses, respectively. Note that `--separate-threads=yes` is not
strictly necessary to be able to trace threads. But, if they are separated,
Iai-Callgrind can collect and display the metrics for each thread. Due to the
way `callgrind` applies [data collection options] like `--toggle-collect`,
`--collect-atstart`, ... further configuration is needed in library benchmarks.

To actually see the collected metrics in the terminal output for all threads
and/or subprocesses you can switch on `OutputFormat::show_intermediate`:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn find_primes_multi_thread(_: u64) -> Vec<u64> { vec![]} }
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    OutputFormat
};
use std::hint::black_box;

#[library_benchmark]
fn bench_threads() -> Vec<u64> {
    black_box(my_lib::find_primes_multi_thread(2))
}

library_benchmark_group!(name = my_group; benchmarks = bench_threads);
# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .show_intermediate(true)
        );
    library_benchmark_groups = my_group
);
# }
```

The best method for benchmarking threads and subprocesses depends heavily on
your code. So, rather than suggesting a single "best" method for benchmarking
threads and subprocesses, this chapter will run through various possible
approaches and try to highlight the pros and cons of each.

## Multi-threaded applications

`Callgrind` treats each thread and process as a separate unit and it applies
data collection options to each unit. In library benchmarks the [entry
point](./custom_entry_point.md) (or the default toggle) for `callgrind` is per
default set to the benchmark function with the help of the `--toggle-collect`
option. Setting `--toggle-collect` also automatically sets
`--collect-atstart=no`. If not further customized for a benchmarked
multi-threaded function, these options cause the metrics for the spawned threads
to be zero. This happens since each thread is a separate unit with
`--collect-atstart=no` and the default toggle applied to the units. The default
toggle is set to the benchmark function and does not hook into any function in
the thread, so the metrics are zero.

There are multiple ways to customize the default behaviour and actually measure
the threads. For the following examples, we're using the benchmark and library
code below to show the different customization options assuming this code lives
in a benchmark file `benches/lib_bench_threads.rs`

```rust
# extern crate iai_callgrind;
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    OutputFormat
};
use std::hint::black_box;

/// Suppose this is your library
pub mod my_lib {
    /// Return true if `num` is a prime number
    pub fn is_prime(num: u64) -> bool {
        if num <= 1 {
            return false;
        }

        for i in 2..=(num as f64).sqrt() as u64 {
            if num % i == 0 {
                return false;
            }
        }

        true
    }

    /// Find and return all prime numbers in the inclusive range `low` to `high`
    pub fn find_primes(low: u64, high: u64) -> Vec<u64> {
        (low..=high).filter(|n| is_prime(*n)).collect()
    }

    /// Return the prime numbers in the range `0..(num_threads * 10000)`
    pub fn find_primes_multi_thread(num_threads: usize) -> Vec<u64> {
        let mut handles = vec![];
        let mut low = 0;
        for _ in 0..num_threads {
            let handle = std::thread::spawn(move || find_primes(low, low + 10000));
            handles.push(handle);

            low += 10000;
        }

        let mut primes = vec![];
        for handle in handles {
            let result = handle.join();
            primes.extend(result.unwrap())
        }

        primes
    }
}

#[library_benchmark]
#[bench::two_threads(2)]
fn bench_threads(num_threads: usize) -> Vec<u64> {
    black_box(my_lib::find_primes_multi_thread(num_threads))
}

library_benchmark_group!(name = my_group; benchmarks = bench_threads);
# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .show_intermediate(true)
        );
    library_benchmark_groups = my_group
);
# }
```

Running this benchmark with `cargo bench` will present you with the following
terminal output:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2097219 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                       <b>27305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66353</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>341</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>539</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67233</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86923</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2097219 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2097219 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                       <b>27305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66353</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>341</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>539</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67233</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86923</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 1.19222s</code></pre>

As you can see, the counts for the threads `2` and `3` (our spawned threads) are
all zero.

### Measuring threads using toggles

At a first glance, setting a toggle to the function in the thread seems to be
easiest way and can be done like so:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn find_primes_multi_thread(_: usize) -> Vec<u64> { vec![] }}
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    EntryPoint, Callgrind
};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--toggle-collect=lib_bench_threads::my_lib::find_primes"]))
)]
#[bench::two_threads(2)]
fn bench_threads(num_threads: usize) -> Vec<u64> {
    black_box(my_lib::find_primes_multi_thread(num_threads))
}
# library_benchmark_group!(name = my_group; benchmarks = bench_threads);
# fn main() {
# main!(library_benchmark_groups = my_group);
# }
```

This approach may or may not work, depending on whether the compiler inlines the
target function of the `--toggle-collect` argument or not. This is the same
problem as with [custom entry
points](./custom_entry_point.md#pitfall-inlined-functions). As can be seen
below, the compiler has chosen to inline `find_primes` and the metrics for the
threads are still zero:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2620776 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                       <b>27372</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66431</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>343</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>538</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67312</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86976</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2620776 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2620776 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                       <b>27372</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66431</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>343</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>538</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67312</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86976</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 1.19222s</code></pre>

Just to show what would happen if the compiler does not inline the `find_primes`
method, we temporarily annotate it with `#[inline(never)]`:

```rust
/// Find and return all prime numbers in the inclusive range `low` to `high`
# fn is_prime(_: u64) -> bool { true }
#[inline(never)]
pub fn find_primes(low: u64, high: u64) -> Vec<u64> {
    (low..=high).filter(|n| is_prime(*n)).collect()
}
```

Now, running the benchmark does show the desired metrics:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2661917 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                       <b>27372</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66431</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>343</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>538</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67312</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86976</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2661917 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>2460503</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>2534938</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>12</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>186</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>2535136</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>2541508</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2661917 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>3650410</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>3724286</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>4</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>130</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>3724420</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>3728856</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                     <b>6138285</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>6325655</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>359</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>854</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>6326868</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>6357340</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 1.19222s</code></pre>

But, annotating functions with `#[inline(never)]` in production code is usually
not an option and preventing the compiler from doing its job is not the
preferred way to make a benchmark work. The truth is, there is no way to make
the `--toggle-collect` argument work for all cases and it heavily depends on the
choices of the compiler depending on your code.

Another way to get the thread metrics is to set `--collect-atstart=yes` and turn
off the `EntryPoint`:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn find_primes_multi_thread(_: usize) -> Vec<u64> { vec![] }}
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    EntryPoint, Callgrind
};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--collect-atstart=yes"])
            .entry_point(EntryPoint::None)
        )
)]
#[bench::two_threads(2)]
fn bench_threads(num_threads: usize) -> Vec<u64> {
    black_box(my_lib::find_primes_multi_thread(num_threads))
}
# library_benchmark_group!(name = my_group; benchmarks = bench_threads);
# fn main() {
# main!(library_benchmark_groups = my_group);
# }
```

But, the metrics of the main thread will include all the setup (and teardown)
code from the benchmark executable (so the instructions of the main thread go up
from `27372` to `404425`):

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2697019 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                      <b>404425</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                           <b>570186</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                             <b>1307</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                            <b>4856</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                  <b>576349</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                  <b>746681</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2697019 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>2466864</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>2543314</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>81</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>409</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>2543804</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>2558034</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2697019 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>3656729</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>3732802</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>31</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>201</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>3733034</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>3739992</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                     <b>6528018</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>6846302</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                             <b>1419</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                            <b>5466</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>6853187</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>7044707</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

Additionally, expect a lot of metric changes if the benchmarks itself are
changed. However, if the metrics of the main thread are not significant compared
to the total, this might be an applicable (last) choice.

There is another more reliable way as shown below in the next section.

### Measuring threads using client requests

The perhaps most reliable and flexible way to measure threads is using [client
requests](../../client_requests.md). The downside is that you have to put some
benchmark code into your production code. But, if you followed the installation
instructions in [client requests](../../client_requests.md), this additional
code is only compiled in benchmarks, not in your final production-ready library.

Using the callgrind client request, we adjust the threads in the
`find_primes_multi_thread` function like so:

```rust
# fn find_primes(_a: u64, _b: u64) -> Vec<u64> { vec![] }
# extern crate iai_callgrind;
use iai_callgrind::client_requests::callgrind;

/// Return the prime numbers in the range `0..(num_threads * 10000)`
pub fn find_primes_multi_thread(num_threads: usize) -> Vec<u64> {
    let mut handles = vec![];
    let mut low = 0;
    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || {
            callgrind::toggle_collect();
            let result = find_primes(low, low + 10000);
            callgrind::toggle_collect();
            result
        });
        handles.push(handle);

        low += 10000;
    }

    let mut primes = vec![];
    for handle in handles {
        let result = handle.join();
        primes.extend(result.unwrap())
    }

    primes
}
```

and running the same benchmark now will show the collected metrics of the
threads:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2149242 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                       <b>27305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>66352</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>344</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>537</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>67233</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>86867</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2149242 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>2460501</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>2534935</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>13</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>185</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>2535133</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>2541475</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2149242 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>3650408</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>3724285</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>1</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>131</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>3724417</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>3728875</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                     <b>6138214</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>6325572</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                              <b>358</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>853</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>6326783</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>6357217</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

Using the client request toggles is very flexible since you can put the
`iai_callgrind::client_requests::callgrind::toggle_collect` instructions
anywhere in the threads. In this example, we just have a single function in the
thread, but if your threads consist of more than just a single function, you can
easily exclude uninteresting parts from the final measurements.

If you want to prevent the code of the main thread from being measured, you can
use the following:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn find_primes_multi_thread(_: usize) -> Vec<u64> { vec![] }}
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    EntryPoint, Callgrind
};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--collect-atstart=no"])
            .entry_point(EntryPoint::None)
        )
)]
#[bench::two_threads(2)]
fn bench_threads(num_threads: usize) -> Vec<u64> {
    black_box(my_lib::find_primes_multi_thread(num_threads))
}
# library_benchmark_group!(name = my_group; benchmarks = bench_threads);
# fn main() {
# main!(library_benchmark_groups = my_group);
# }
```

Setting the `EntryPoint::None` disables the default toggle but also
`--collect-atstart=no`, which is why we have to set the option manually.
Altogether, running the benchmark will show:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_threads::my_group::bench_threads</span> <span style="color:#0AA">two_threads</span><span style="color:#0AA">:</span><b><span style="color:#00A">2</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2251257 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2251257 thread: 2 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>2460501</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>2534935</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>11</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>187</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>2535133</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>2541535</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 2251257 thread: 3 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_threads-b85159a94ccb3851</span></b>
<span style="color:#555">  </span>Instructions:                     <b>3650408</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>3724282</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>4</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>131</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>3724417</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>3728887</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                     <b>6110909</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                          <b>6259217</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>15</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>318</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                 <b>6259550</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                 <b>6270422</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

## Multi-process applications

Measuring multi-process applications is in principal not that different from
multi-threaded applications since subprocesses are just like threads separate
units. As for threads, the [data collection options] are applied to subprocesses
separately from the main process.

Note there are multiple [valgrind command-line
arguments](https://valgrind.org/docs/manual/manual-core.html#manual-core.basicopts)
that can disable the collection of metrics for uninteresting subprocesses, for
example subprocesses that are spawned by your library function but are not part
of your library/binary crate.

For the following examples suppose the code below is the `cat` binary and part
of a crate (so we can use
[`env!("CARGO_BIN_EXE_cat")`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)):

```rust
use std::fs::File;
use std::io::{copy, stdout, BufReader, BufWriter, Write};

# fn main() {
fn main() {
    let mut args_iter = std::env::args().skip(1);
    let file_arg = args_iter.next().expect("File argument should be present");

    let file = File::open(file_arg).expect("Opening file should succeed");
    let stdout = stdout().lock();

    let mut writer = BufWriter::new(stdout);
    copy(&mut BufReader::new(file), &mut writer)
        .expect("Printing file to stdout should succeed");

    writer.flush().expect("Flushing writer should succeed");
}
# }
```

The above binary is a very simple version of `cat` taking a single file
argument. The file content is read and dumped to the `stdout`. The following is
the benchmark and library code to show the different options assuming this code
is stored in a benchmark file `benches/lib_bench_subprocess.rs`

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use std::hint::black_box;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
    OutputFormat,
};

/// Suppose this is your library
pub mod my_lib {
    use std::io;
    use std::path::Path;
    use std::process::ExitStatus;

    /// A function executing the crate's binary `cat`
    pub fn cat(file: &Path) -> io::Result<ExitStatus> {
        std::process::Command::new(env!("CARGO_BIN_EXE_cat"))
            .arg(file)
            .status()
    }
}

/// Create a file `/tmp/foo.txt` with some content
fn create_file() -> PathBuf {
    let path = PathBuf::from("/tmp/foo.txt");
    std::fs::write(&path, "some content").unwrap();
    path
}

#[library_benchmark]
#[bench::some(setup = create_file)]
fn bench_subprocess(path: PathBuf) -> io::Result<ExitStatus> {
    black_box(my_lib::cat(&path))
}

library_benchmark_group!(name = my_group; benchmarks = bench_subprocess);
# fn main() {
main!(
    config = LibraryBenchmarkConfig::default()
        .output_format(OutputFormat::default()
            .show_intermediate(true)
        );
    library_benchmark_groups = my_group
);
# }
```

Running the above benchmark with `cargo bench` results in the following terminal
output:

<pre><code class="hljs"><span style="color:#0A0">lib_bench_subprocess::my_group::bench_subprocess</span> <span style="color:#0AA">some</span><span style="color:#0AA">:</span><b><span style="color:#00A">create_file()</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3141785 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_subprocess-a1b2e1eac5125819</span></b>
<span style="color:#555">  </span>Instructions:                        <b>4467</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>6102</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>17</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>186</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>6305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>12697</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3141786 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/cat /tmp/foo.txt</span></b>
<span style="color:#555">  </span>Instructions:                           <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                               <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                       <b>0</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                        <b>4467</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>6102</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>17</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>186</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>6305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>12697</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

As expected, the `cat` subprocess is not measured and the metrics are zero for
the same reasons as the initial measurement of threads.

### Measuring subprocesses using toggles

The great advantage over measuring threads is that each process has a main
function that is not inlined by the compiler and can serve as a reliable hook
for the `--toggle-collect` argument so the following adaption to the above
benchmark will just work:

```rust
# extern crate iai_callgrind;
# mod my_lib {
# use std::{io, path::Path, process::ExitStatus};
# pub fn cat(_: &Path) -> io::Result<ExitStatus> {
#    std::process::Command::new("some").status()
# }}
# fn create_file() -> PathBuf { PathBuf::from("some") }
# use std::hint::black_box;
# use std::io;
# use std::path::PathBuf;
# use std::process::ExitStatus;
# use iai_callgrind::{
#    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
#    OutputFormat, Callgrind
# };
#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::with_args(["--toggle-collect=cat::main"]))
)]
#[bench::some(setup = create_file)]
fn bench_subprocess(path: PathBuf) -> io::Result<ExitStatus> {
    black_box(my_lib::cat(&path))
}
# library_benchmark_group!(name = my_group; benchmarks = bench_subprocess);
# fn main() {
# main!(library_benchmark_groups = my_group);
# }
```

producing the desired output

<pre><code class="hljs"><span style="color:#0A0">lib_bench_subprocess::my_group::bench_subprocess</span> <span style="color:#0AA">some</span><span style="color:#0AA">:</span><b><span style="color:#00A">create_file()</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3324117 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_subprocess-a1b2e1eac5125819</span></b>
<span style="color:#555">  </span>Instructions:                        <b>4475</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>6112</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>14</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>187</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>6313</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>12727</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3324119 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/cat /tmp/foo.txt</span></b>
<span style="color:#555">  </span>Instructions:                        <b>4019</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>5575</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>12</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>167</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>5754</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>11480</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                        <b>8494</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                            <b>11687</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>26</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>354</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                   <b>12067</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>24207</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

### Measuring subprocesses using client requests

Naturally, client requests can also be used to measure subprocesses. The
callgrind client requests are added to the code of the `cat` binary:

```rust
# extern crate iai_callgrind;
use std::fs::File;
use std::io::{copy, stdout, BufReader, BufWriter, Write};
use iai_callgrind::client_requests::callgrind;

# fn main() {
fn main() {
    let mut args_iter = std::env::args().skip(1);
    let file_arg = args_iter.next().expect("File argument should be present");

    callgrind::toggle_collect();
    let file = File::open(file_arg).expect("Opening file should succeed");
    let stdout = stdout().lock();

    let mut writer = BufWriter::new(stdout);
    copy(&mut BufReader::new(file), &mut writer)
        .expect("Printing file to stdout should succeed");

    writer.flush().expect("Flushing writer should succeed");
    callgrind::toggle_collect();
}
# }
```

For the purpose of this example we decided that measuring the parsing of the
command-line-arguments is not interesting for us and excluded it from the
collected metrics. The benchmark itself is reverted to its original state
without the toggle:

```rust
# extern crate iai_callgrind;
# mod my_lib {
# use std::{io, path::Path, process::ExitStatus};
# pub fn cat(_: &Path) -> io::Result<ExitStatus> {
#    std::process::Command::new("some").status()
# }}
# fn create_file() -> PathBuf { PathBuf::from("some") }
# use std::hint::black_box;
# use std::io;
# use std::path::PathBuf;
# use std::process::ExitStatus;
# use iai_callgrind::{
#    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
#    OutputFormat,
# };
#[library_benchmark]
#[bench::some(setup = create_file)]
fn bench_subprocess(path: PathBuf) -> io::Result<ExitStatus> {
    black_box(my_lib::cat(&path))
}
# library_benchmark_group!(name = my_group; benchmarks = bench_subprocess);
# fn main() {
# main!(library_benchmark_groups = my_group);
# }
```

Now, running the benchmark shows

<pre><code class="hljs"><span style="color:#0A0">lib_bench_subprocess::my_group::bench_subprocess</span> <span style="color:#0AA">some</span><span style="color:#0AA">:</span><b><span style="color:#00A">create_file()</span></b>
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3421822 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/deps/lib_bench_subprocess-a1b2e1eac5125819</span></b>
<span style="color:#555">  </span>Instructions:                        <b>4467</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>6102</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>17</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>186</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>6305</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>12697</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>pid: 3421823 thread: 1 part: 1</b>        |N/A
<span style="color:#555">  </span>Command:             <b><span style="color:#00A">target/release/cat /tmp/foo.txt</span></b>
<span style="color:#555">  </span>Instructions:                        <b>2429</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>3406</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                                <b>8</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>138</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>3552</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                    <b>8276</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span><span style="color:#A50">##</span> <b>Total</b>
<span style="color:#555">  </span>Instructions:                        <b>6896</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L1 Hits:                             <b>9508</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>L2 Hits:                               <b>25</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>RAM Hits:                             <b>324</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Total read+write:                    <b>9857</b>|N/A                  (<span style="color:#555">*********</span>)
<span style="color:#555">  </span>Estimated Cycles:                   <b>20973</b>|N/A                  (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

As expected, the metrics for the `cat` binary are a little bit lower since we
skipped measuring the parsing of the command-line arguments.

[data collection options]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options.collection
