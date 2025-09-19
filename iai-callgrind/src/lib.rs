//! Gungraun is a benchmarking framework/harness which primarily uses [Valgrind's
//! Callgrind](https://valgrind.org/docs/manual/cl-manual.html) to provide extremely accurate and
//! consistent measurements of Rust code, making it perfectly suited to run in environments like a
//! CI. Gungraun is flexible and despite its name it's possible to run Cachegrind or any other
//! Valgrind tool like DHAT in addition to or instead of Callgrind.
//!
//! The [online guide][Guide] contains all the details to start profiling with Gungraun.
//!
//! # Table of contents
//! - [Characteristics](#characteristics)
//! - [Benchmarking](#benchmarking)
//!   - [Library Benchmarks](#library-benchmarks)
//!     - [Important Default Behavior](#important-default-behavior)
//!     - [Quickstart](#quickstart-library-benchmarks)
//!     - [Configuration](#configuration-library-benchmarks)
//!   - [Binary Benchmarks](#binary-benchmarks)
//!     - [Important default behavior](#important-default-behavior)
//!     - [Quickstart](#quickstart-binary-benchmarks)
//!     - [Configuration](#configuration-binary-benchmarks)
//! - [Valgrind Tools](#valgrind-tools)
//! - [Client Requests](#client-requests)
//! - [Flamegraphs](#flamegraphs)
//!
//! ## Characteristics
//!
//! - __Precision__: High-precision measurements allow you to reliably detect very small
//!   optimizations of your code
//! - __Consistency__: Gungraun can take accurate measurements even in virtualized CI environments
//! - __Performance__: Since Gungraun only executes a benchmark once, it is typically a lot faster
//!   to run than benchmarks measuring the execution and wall-clock time
//! - __Regression__: Gungraun reports the difference between benchmark runs to make it easy to spot
//!   detailed performance regressions and improvements.
//! - __CPU and Cache Profiling__: Gungraun generates a Callgrind profile of your code while
//!   benchmarking, so you can use Callgrind-compatible tools like
//!   [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
//!   or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the
//!   results in detail.
//! - __Memory Profiling__: You can run other Valgrind tools like [DHAT: a dynamic heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html)
//!   and [Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html) with the
//!   Gungraun benchmarking framework. Their profiles are stored next to the callgrind profiles and
//!   are ready to be examined with analyzing tools like `dh_view.html`, `ms_print` and others.
//! - __Visualization__: Gungraun is capable of creating regular and differential flamegraphs from
//!   the Callgrind output format.
//! - __Valgrind Client Requests__: Support of zero overhead [Valgrind Client Requests](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)
//!   (compared to native valgrind client requests overhead) on many targets
//! - __Stable-compatible__: Benchmark your code without installing nightly Rust
//!
//! ## Benchmarking
//!
//! `iai-callgrind` can be divided into two sections: Benchmarking the library and
//! its public functions and benchmarking of the binaries of a crate.
//!
//! ### Library Benchmarks
//!
//! Use this scheme of the [`main`] macro if you want to benchmark functions of your
//! crate's library.
//!
//! #### Important default behavior
//!
//! The environment variables are cleared before running a library benchmark. See also the
//! Configuration section below if you need to change that behavior.
//!
//! #### Quickstart (#library-benchmarks)
//!
//! ```rust
//! use iai_callgrind::{
//!     library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig
//! };
//! use std::hint::black_box;
//!
//! // Our function we want to test. Just assume this is a public function in your
//! // library.
//! fn bubble_sort(mut array: Vec<i32>) -> Vec<i32> {
//!     for i in 0..array.len() {
//!         for j in 0..array.len() - i - 1 {
//!             if array[j + 1] < array[j] {
//!                 array.swap(j, j + 1);
//!             }
//!         }
//!     }
//!     array
//! }
//!
//! // This function is used to create a worst case array we want to sort with our
//! // implementation of bubble sort
//! fn setup_worst_case_array(start: i32) -> Vec<i32> {
//!     if start.is_negative() {
//!         (start..0).rev().collect()
//!     } else {
//!         (0..start).rev().collect()
//!     }
//! }
//!
//! // The #[library_benchmark] attribute let's you define a benchmark function which you
//! // can later use in the `library_benchmark_groups!` macro.
//! #[library_benchmark]
//! fn bench_bubble_sort_empty() -> Vec<i32> {
//!     // The `black_box` is needed to tell the compiler to not optimize what's inside
//!     // black_box or else the benchmarks might return inaccurate results.
//!     black_box(bubble_sort(black_box(vec![])))
//! }
//!
//! // This benchmark uses the `bench` attribute to setup benchmarks with different
//! // setups. The big advantage is, that the setup costs and event counts aren't
//! // attributed to the benchmark (and opposed to the old api we don't have to deal with
//! // callgrind arguments, toggles, ...)
//! #[library_benchmark]
//! #[bench::empty(vec![])]
//! #[bench::worst_case_6(vec![6, 5, 4, 3, 2, 1])]
//! // Function calls are fine too
//! #[bench::worst_case_4000(setup_worst_case_array(4000))]
//! // The argument of the benchmark function defines the type of the argument from the
//! // `bench` cases.
//! fn bench_bubble_sort(array: Vec<i32>) -> Vec<i32> {
//!     // Note `array` is not put in a `black_box` because that's already done for you.
//!     black_box(bubble_sort(array))
//! }
//!
//! // You can use the `benches` attribute to specify multiple benchmark runs in one go. You can
//! // specify multiple `benches` attributes or mix the `benches` attribute with `bench`
//! // attributes.
//! #[library_benchmark]
//! // This is the simple form. Each `,`-separated element is another benchmark run and is
//! // passed to the benchmarking function as parameter. So, this is the same as specifying
//! // two `#[bench]` attributes #[bench::multiple_0(vec![1])] and #[bench::multiple_1(vec![5])].
//! #[benches::multiple(vec![1], vec![5])]
//! // You can also use the `args` argument to achieve the same. Using `args` is necessary if you
//! // also want to specify a `config` or `setup` function.
//! #[benches::with_args(args = [vec![1], vec![5]], config = LibraryBenchmarkConfig::default())]
//! // Usually, each element in `args` is passed directly to the benchmarking function. You can
//! // instead reroute them to a `setup` function. In that case the (black boxed) return value of
//! // the setup function is passed as parameter to the benchmarking function.
//! #[benches::with_setup(args = [1, 5], setup = setup_worst_case_array)]
//! fn bench_bubble_sort_with_benches_attribute(input: Vec<i32>) -> Vec<i32> {
//!     black_box(bubble_sort(input))
//! }
//!
//! // A benchmarking function with multiple parameters requires the elements to be specified as
//! // tuples.
//! #[library_benchmark]
//! #[benches::multiple((1, 2), (3, 4))]
//! fn bench_bubble_sort_with_multiple_parameters(a: i32, b: i32) -> Vec<i32> {
//!     black_box(bubble_sort(black_box(vec![a, b])))
//! }
//!
//! // A group in which we can put all our benchmark functions
//! library_benchmark_group!(
//!     name = bubble_sort_group;
//!     benchmarks =
//!         bench_bubble_sort_empty,
//!         bench_bubble_sort,
//!         bench_bubble_sort_with_benches_attribute,
//!         bench_bubble_sort_with_multiple_parameters
//! );
//!
//! # fn main() {
//! // Finally, the mandatory main! macro which collects all `library_benchmark_groups`.
//! // The main! macro creates a benchmarking harness and runs all the benchmarks defined
//! // in the groups and benches.
//! main!(library_benchmark_groups = bubble_sort_group);
//! # }
//! ```
//!
//! Note that it is important to annotate the benchmark functions with
//! [`#[library_benchmark]`](crate::library_benchmark).
//!
//! ### Configuration (#library-benchmarks)
//!
//! It's possible to configure some of the behavior of `iai-callgrind`. See the docs of
//! [`crate::LibraryBenchmarkConfig`] for more details. Configure library benchmarks at
//! top-level with the [`crate::main`] macro, at group level within the
//! [`crate::library_benchmark_group`], at [`crate::library_benchmark`] level
//!
//! and at `bench` level:
//!
//! ```rust
//! # use iai_callgrind::{LibraryBenchmarkConfig, library_benchmark};
//! #[library_benchmark]
//! #[bench::some_id(args = (1, 2), config = LibraryBenchmarkConfig::default())]
//! // ...
//! # fn some_func(first: u8, second: u8) -> u8 {
//! #    first + second
//! # }
//! # fn main() {}
//! ```
//!
//! The config at `bench` level overwrites the config at `library_benchmark` level. The config at
//! `library_benchmark` level overwrites the config at group level and so on. Note that
//! configuration values like `envs` are additive and don't overwrite configuration values of higher
//! levels.
//!
//! See also the docs of [`crate::library_benchmark_group`]. The [online guide][Guide] includes more
//! explanations, common recipes and examples.
//!
//! ### Binary Benchmarks
//!
//! Use this scheme of the [`main`] macro to benchmark one or more binaries of your crate (or any
//! other executable). The documentation for setting up binary benchmarks with the
//! `binary_benchmark_group` macro can be found in the docs of [`crate::binary_benchmark_group`].
//!
//! #### Important default behavior
//!
//! Per default, all binary benchmarks run with the environment variables cleared. See also
//! [`crate::BinaryBenchmarkConfig::env_clear`] for how to change this behavior.
//!
//! #### Quickstart (#binary-benchmarks)
//!
//! There are two apis to set up binary benchmarks, but we only describe the high-level api using
//! the [`#[binary_benchmark]`](`crate::binary_benchmark`) attribute here. See the docs of
//! [`binary_benchmark_group`] for more details about the low level api. The `#[binary_benchmark]`
//! attribute works almost the same as the `#[library_benchmark]` attribute. You will find the same
//! parameters `setup`, `teardown`, `config`, etc. in `#[binary_benchmark]` as in
//! `#[library_benchmark]` and the inner attributes `#[bench]`, `#[benches]`. But, there are also
//! substantial (differences)[#differences-to-library-benchmarks].
//!
//! Suppose your crate's binaries are named `my-foo` and `my-bar`
//!
//! ```rust
//! # macro_rules! env { ($m:tt) => {{ "/some/path" }} }
//! use iai_callgrind::{
//!     main, binary_benchmark, binary_benchmark_group,
//! };
//! use std::path::PathBuf;
//! use std::ffi::OsString;
//!
//! // In binary benchmarks there's no need to return a value from the setup function
//! fn my_setup() {
//!     println!("Put code in here which will be run before the actual command");
//! }
//!
//! #[binary_benchmark]
//! #[bench::just_a_fixture("benches/fixture.json")]
//! // First big difference to library benchmarks! `my_setup` is not evaluated right away and the
//! // return value of `my_setup` is not used as input for the `bench_foo` function. Instead,
//! // `my_setup()` is executed before the execution of the `Command`.
//! #[bench::with_other_fixture_and_setup(args = ("benches/other_fixture.txt"), setup = my_setup())]
//! #[benches::multiple("benches/fix_1.txt", "benches/fix_2.txt")]
//! // All functions annotated with `#[binary_benchmark]` need to return a `iai_callgrind::Command`
//! fn bench_foo(path: &str) -> iai_callgrind::Command {
//!     let path: PathBuf = path.into();
//!     // We can put any code in here which is needed to configure the `Command`.
//!     let stdout = if path.extension().unwrap() == "txt" {
//!         iai_callgrind::Stdio::Inherit
//!     } else {
//!         iai_callgrind::Stdio::File(path.with_extension("out"))
//!     };
//!     // Configure the command depending on the arguments passed to this function and the code
//!     // above
//!     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
//!         .stdout(stdout)
//!         .arg(path)
//!         .build()
//! }
//!
//! #[binary_benchmark]
//! // The id just needs to be unique within the same `#[binary_benchmark]`, so we can reuse
//! // `just_a_fixture` if we want to
//! #[bench::just_a_fixture("benches/fixture.json")]
//! // The function can be generic, too.
//! fn bench_bar<P>(path: P) -> iai_callgrind::Command
//! where
//!    P: Into<OsString>
//! {
//!     iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-bar"))
//!         .arg(path)
//!         .build()
//! }
//!
//! // Put all `#[binary_benchmark]` annotated functions you want to benchmark into the `benchmarks`
//! // section of this macro
//! binary_benchmark_group!(
//!     name = my_group;
//!     benchmarks = bench_foo, bench_bar
//! );
//!
//! # fn main() {
//! // As last step specify all groups you want to benchmark in the macro argument
//! // `binary_benchmark_groups`. As the binary_benchmark_group macro, the main macro is
//! // always needed and finally expands to a benchmarking harness
//! main!(binary_benchmark_groups = my_group);
//! # }
//! ```
//!
//! #### Differences to library benchmarks
//!
//! As opposed to library benchmarks the function annotated with the `binary_benchmark` attribute
//! always returns a `iai_callgrind::Command`. More specifically, this function is not a benchmark
//! function, since we don't benchmark functions anymore but [`Command`]s instead which are the
//! return value of the [`#[binary_benchmark]`](crate::binary_benchmark) function.
//!
//! This change has far-reaching consequences but also simplifies things. Since the function itself
//! is not benchmarked you can put any code into this function, and it does not influence the
//! benchmark of the [`Command`] itself. However, this function is run only once to __build__ the
//! [`Command`] and when we collect all commands and its configuration to be able to actually
//! __execute__ the [`Command`]s later in the benchmark runner. Whichever code you want to run
//! before the [`Command`] is executed has to go into the `setup`. And, into `teardown` for code you
//! want to run after the execution of the [`Command`].
//!
//! In library benchmarks the `setup` argument only takes a path to a function, more specifically
//! the function pointer. In binary benchmarks however, the `setup` (and `teardown`) parameters of
//! the [`#[binary_benchmark]`](crate::binary_benchmark), `#[bench]` and `#[benches]` attribute
//! take expressions which includes function calls for example `setup = my_setup()`. Only in the
//! special case that the expression is a function pointer, we pass the `args` of the `#[bench]` and
//! `#[benches]` attributes into the `setup`, `teardown` __and__ the function itself. Also, these
//! expressions are not executed right away but in a separate process before the [`Command`] is
//! executed. This is the main reason why the return value of the setup function is simply ignored
//! and not routed back into the benchmark function as it would be the case in library benchmarks.
//! We simply don't need to. To sum it up, put code you need to configure the [`Command`] into the
//! annotated function and code you need to execute before (after) the execution of the [`Command`]
//! into the `setup` (`teardown`).
//!
//! #### Configuration (#binary-benchmarks)
//!
//! Much like the configuration of library benchmarks (See above) it's possible to configure binary
//! benchmarks at top-level in the `main!` macro and at group-level in the
//! `binary_benchmark_groups!` with the `config = ...;` argument. In contrast to library benchmarks,
//! binary benchmarks can be also configured at a lower and last level in [`Command`] directly.
//!
//! For further details see the section about binary benchmarks of the [`crate::main`] docs the docs
//! of [`crate::binary_benchmark_group`] and [`Command`]. The [guide][Guide] of this crate includes
//! a more thorough documentation with additional examples.
//!
//! ## Valgrind Tools
//!
//! In addition to or instead of the default Callgrind tool, you can use the Gungraun framework
//! to run other Valgrind profiling tools like `DHAT`, `Massif`, the experimental `BBV` and even
//! `Cachegrind`. But, also `Memcheck`, `Helgrind` and `DRD` if you need to check memory and thread
//! safety of benchmarked code. See the [Valgrind User
//! Manual](https://valgrind.org/docs/manual/manual.html) for details and command line arguments.
//! The additional tools can be specified in [`LibraryBenchmarkConfig::tool`],
//! [`BinaryBenchmarkConfig::tool`]. For example to run `DHAT` for all library benchmarks:
//!
//! ```rust
//! # use iai_callgrind::{library_benchmark, library_benchmark_group};
//! use iai_callgrind::{main, LibraryBenchmarkConfig, Dhat};
//! # #[library_benchmark]
//! # fn some_func() {}
//! # library_benchmark_group!(name = some_group; benchmarks = some_func);
//! # fn main() {
//! main!(
//!     config = LibraryBenchmarkConfig::default()
//!                 .tool(Dhat::default());
//!     library_benchmark_groups = some_group
//! );
//! # }
//! ```
//!
//! If you're just interested in for example DHAT metrics for one or more specific benchmarks you
//! can change the default tool wherever a configuration can be specified. Here in `main!`:
//!
//! ```rust
//! # use iai_callgrind::{library_benchmark, library_benchmark_group};
//! use iai_callgrind::{main, LibraryBenchmarkConfig, ValgrindTool};
//! # #[library_benchmark]
//! # fn some_func() {}
//! # library_benchmark_group!(name = some_group; benchmarks = some_func);
//! # fn main() {
//! main!(
//!     config = LibraryBenchmarkConfig::default()
//!                 .default_tool(ValgrindTool::DHAT);
//!     library_benchmark_groups = some_group
//! );
//! # }
//! ```
//!
//! ## Client requests
//!
//! `iai-callgrind` supports valgrind client requests. See the documentation of the
//! [`client_requests`] module for all the details.
//!
//! ## Flamegraphs
//!
//! Flamegraphs are opt-in and can be created if you pass a [`FlamegraphConfig`] to the
//! [`Callgrind::flamegraph`]. Callgrind flamegraphs are meant as a complement to valgrind's
//! visualization tools `callgrind_annotate` and `kcachegrind`.
//!
//! Callgrind flamegraphs show the inclusive costs for functions and a specific event type, much
//! like `callgrind_annotate` does but in a nicer (and clickable) way. Especially, differential
//! flamegraphs facilitate a deeper understanding of code sections which cause a bottleneck or a
//! performance regressions etc.
//!
//! The produced flamegraph svg files are located next to the respective callgrind output file in
//! the `target/iai` directory.
//!
//! [Guide]: https://gungraun.github.io/gungraun/latest/html/intro.html

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]

#[cfg(feature = "default")]
#[doc(hidden)]
pub mod __internal;
#[cfg(feature = "default")]
mod bin_bench;
#[cfg(feature = "client_requests_defs")]
pub mod client_requests;
#[cfg(feature = "default")]
mod common;
#[cfg(feature = "default")]
mod lib_bench;
#[cfg(feature = "default")]
mod macros;
#[cfg(feature = "default")]
pub use bin_bench::{
    Bench, BenchmarkId, BinaryBenchmark, BinaryBenchmarkConfig, BinaryBenchmarkGroup, Command,
    Delay, Sandbox,
};
#[cfg(feature = "default")]
pub use bincode;
#[cfg(feature = "default")]
pub use common::{
    Bbv, Cachegrind, Callgrind, Dhat, Drd, FlamegraphConfig, Helgrind, Massif, Memcheck,
    OutputFormat,
};
#[cfg(feature = "client_requests_defs")]
pub use cty;
#[cfg(feature = "default")]
pub use iai_callgrind_macros::{binary_benchmark, library_benchmark};
// Only add enums here. Do not re-export structs from the runner api directly. See the
// documentation in `__internal::mod` for more details.
#[cfg(feature = "default")]
pub use iai_callgrind_runner::api::{
    CachegrindMetric, CachegrindMetrics, CallgrindMetrics, DelayKind, DhatMetric, DhatMetrics,
    Direction, EntryPoint, ErrorMetric, EventKind, ExitWith, FlamegraphKind, Limit, Pipe, Stdin,
    Stdio, ValgrindTool,
};
#[cfg(feature = "default")]
pub use lib_bench::LibraryBenchmarkConfig;
