//! Iai-Callgrind is a benchmarking framework/harness which primarily uses [Valgrind's
//! Callgrind](https://valgrind.org/docs/manual/cl-manual.html) and the other Valgrind tools to
//! provide extremely accurate and consistent measurements of Rust code, making it perfectly suited
//! to run in environments like a CI.
//!
//! # Table of contents
//! - [Characteristics](#characteristics)
//! - [Benchmarking](#benchmarking)
//!   - [Library Benchmarks](#library-benchmarks)
//!     - [Important Default Behavior](#important-default-behavior)
//!     - [Quickstart](#quickstart-library-benchmarks)
//!     - [Configuration](#configuration-library-benchmarks)
//!   - [Binary Benchmarks](#binary-benchmarks)
//!     - [Temporary workspace and other important default
//!       behavior](#temporary-workspace-and-other-important-default-behavior)
//!     - [Quickstart](#quickstart-binary-benchmarks)
//!     - [Configuration](#configuration-binary-benchmarks)
//! - [Valgrind Tools](#valgrind-tools)
//! - [Client Requests](#client-requests)
//! - [Flamegraphs](#flamegraphs)
//!
//! ## Characteristics
//! - __Precision__: High-precision measurements allow you to reliably detect very small
//! optimizations of your code
//! - __Consistency__: Iai-Callgrind can take accurate measurements even in virtualized CI
//! environments
//! - __Performance__: Since Iai-Callgrind only executes a benchmark once, it is typically a lot
//! faster to run than benchmarks measuring the execution and wall time
//! - __Regression__: Iai-Callgrind reports the difference between benchmark runs to make it easy to
//! spot detailed performance regressions and improvements.
//! - __CPU and Cache Profiling__: Iai-Callgrind generates a Callgrind profile of your code while
//! benchmarking, so you can use Callgrind-compatible tools like
//! [callgrind_annotate](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.callgrind_annotate-options)
//! or the visualizer [kcachegrind](https://kcachegrind.github.io/html/Home.html) to analyze the
//! results in detail.
//! - __Memory Profiling__: You can run other Valgrind tools like [DHAT: a dynamic heap analysis
//! tool](https://valgrind.org/docs/manual/dh-manual.html) and [Massif: a heap
//! profiler](https://valgrind.org/docs/manual/ms-manual.html) with the Iai-Callgrind benchmarking
//! framework. Their profiles are stored next to the callgrind profiles and are ready to be examined
//! with analyzing tools like `dh_view.html`, `ms_print` and others.
//! - __Visualization__: Iai-Callgrind is capable of creating regular and differential flamegraphs
//! from the Callgrind output format.
//! - __Valgrind Client Requests__: Support of zero overhead [Valgrind Client
//! Requests](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)
//! (compared to native valgrind client requests overhead) on many targets
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
//! See also the docs of [`crate::library_benchmark_group`]. The
//! [README](https://github.com/iai-callgrind/iai-callgrind) of this crate includes more explanations,
//! common recipes and some examples.
//!
//! ### Binary Benchmarks
//!
//! Use this scheme of the [`main`] macro to benchmark one or more binaries of your crate. If you
//! really like to, it's possible to benchmark any executable file in the PATH or any executable
//! specified with an absolute path. The documentation for setting up binary benchmarks with the
//! `binary_benchmark_group` macro can be found in the docs of [`crate::binary_benchmark_group`].
//!
//! #### Temporary Workspace and other important default behavior
//!
//! Per default, all binary benchmarks and the `before`, `after`, `setup` and `teardown` functions
//! are executed in a temporary directory. See [`crate::BinaryBenchmarkConfig::sandbox`] for a
//! deeper explanation and how to control and change this behavior. Also, the environment variables
//! of benchmarked binaries are cleared before the benchmark is run. See also
//! [`crate::BinaryBenchmarkConfig::env_clear`] for how to change this behavior.
//!
//! #### Quickstart (#binary-benchmarks)
//!
//! Suppose your crate's binary is named `my-exe` and you have a fixtures directory in
//! `benches/fixtures` with a file `test1.txt` in it:
//!
//! ```rust
//! use iai_callgrind::{
//!     main, binary_benchmark_group, BinaryBenchmarkConfig, BinaryBenchmarkGroup,
//!     Run, Arg, Fixtures
//! };
//!
//! fn my_setup() {
//!     println!("We can put code in here which will be run before each benchmark run");
//! }
//!
//! // We specify a cmd `"my-exe"` for the whole group which is a binary of our crate. This
//! // eliminates the need to specify a `cmd` for each `Run` later on and we can use the
//! // auto-discovery of a crate's binary at group level. We'll also use the `setup` argument
//! // to run a function before each of the benchmark runs.
//! binary_benchmark_group!(
//!     name = my_exe_group;
//!     setup = my_setup;
//!     // This directory will be copied into the root of the sandbox (as `fixtures`)
//!     config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("benches/fixtures"));
//!     benchmark = |"my-exe", group: &mut BinaryBenchmarkGroup| setup_my_exe_group(group));
//!
//! // Working within a macro can be tedious sometimes so we moved the setup code into
//! // this method
//! fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
//!     group
//!         // Setup our first run doing something with our fixture `test1.txt`. The
//!         // id (here `do foo with test1`) of an `Arg` has to be unique within the
//!         // same group
//!         .bench(Run::with_arg(Arg::new(
//!             "do foo with test1",
//!             ["--foo=fixtures/test1.txt"],
//!         )))
//!
//!         // Setup our second run with two positional arguments
//!         .bench(Run::with_arg(Arg::new(
//!             "positional arguments",
//!             ["foo", "foo bar"],
//!         )))
//!
//!         // Our last run doesn't take an argument at all.
//!         .bench(Run::with_arg(Arg::empty("no argument")));
//! }
//!
//! # fn main() {
//! // As last step specify all groups we want to benchmark in the main! macro argument
//! // `binary_benchmark_groups`. The main macro is always needed and finally expands
//! // to a benchmarking harness
//! main!(binary_benchmark_groups = my_exe_group);
//! # }
//! ```
//! #### Configuration (#binary-benchmarks)
//!
//! Much like the configuration of library benchmarks (See above) it's possible to configure binary
//! benchmarks at top-level in the `main!` macro and at group-level in the
//! `binary_benchmark_groups!` with the `config = ...;` argument. In contrast to library benchmarks,
//! binary benchmarks can be configured at a lower and last level within [`Run`] directly.
//!
//! For further details see the section about binary benchmarks of the [`crate::main`] docs the docs
//! of [`crate::binary_benchmark_group`] and [`Run`]. Also, the
//! [README](https://github.com/iai-callgrind/iai-callgrind) of this crate includes some introductory
//! documentation with additional examples.
//!
//! ## Valgrind Tools
//!
//! In addition to the default benchmarks, you can use the Iai-Callgrind framework to run other
//! Valgrind profiling [`Tool`]s like `DHAT`, `Massif` and the experimental `BBV` but also
//! `Memcheck`, `Helgrind` and `DRD` if you need to check memory and thread safety of benchmarked
//! code. See also the [Valgrind User Manual](https://valgrind.org/docs/manual/manual.html) for
//! details and command line arguments. The additional tools can be specified in
//! [`LibraryBenchmarkConfig`], [`BinaryBenchmarkConfig`] or [`Run`]. For example to run `DHAT` for
//! all library benchmarks:
//!
//! ```rust
//! # use iai_callgrind::{library_benchmark, library_benchmark_group};
//! use iai_callgrind::{main, LibraryBenchmarkConfig, Tool, ValgrindTool};
//! # #[library_benchmark]
//! # fn some_func() {}
//! # library_benchmark_group!(name = some_group; benchmarks = some_func);
//! # fn main() {
//! main!(
//!     config = LibraryBenchmarkConfig::default()
//!                 .tool(Tool::new(ValgrindTool::DHAT));
//!     library_benchmark_groups = some_group
//! );
//! # }
//! ```
//!
//! ## Client requests
//!
//! `iai-callgrind` supports valgrind client requests. See the documentation of the
//! [`client_requests`] module.
//!
//! ## Flamegraphs
//!
//! Flamegraphs are opt-in and can be created if you pass a [`FlamegraphConfig`] to the
//! [`BinaryBenchmarkConfig::flamegraph`], [`Run::flamegraph`] or
//! [`LibraryBenchmarkConfig::flamegraph`]. Callgrind flamegraphs are meant as a complement to
//! valgrind's visualization tools `callgrind_annotate` and `kcachegrind`.
//!
//! Callgrind flamegraphs show the inclusive costs for functions and a specific event type, much
//! like `callgrind_annotate` does but in a nicer (and clickable) way. Especially, differential
//! flamegraphs facilitate a deeper understanding of code sections which cause a bottleneck or a
//! performance regressions etc.
//!
//! The produced flamegraph svg files are located next to the respective callgrind output file in
//! the `target/iai` directory.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::default_numeric_fallback)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::fn_to_numeric_cast_any)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::mixed_read_write_in_expression)]
#![warn(clippy::partial_pub_fields)]
#![warn(clippy::rest_pat_in_fully_bound_structs)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_to_string)]
#![warn(clippy::todo)]
#![warn(clippy::try_err)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::unneeded_field_pattern)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "default")]
macro_rules! impl_traits {
    ($src:ty, $dst:ty) => {
        impl From<$src> for $dst {
            fn from(value: $src) -> Self {
                value.0
            }
        }

        impl From<&$src> for $dst {
            fn from(value: &$src) -> Self {
                value.0.clone()
            }
        }

        impl From<&mut $src> for $dst {
            fn from(value: &mut $src) -> Self {
                value.0.clone()
            }
        }

        impl AsRef<$src> for $src {
            fn as_ref(&self) -> &$src {
                self
            }
        }
    };
}

#[cfg(feature = "default")]
mod bin_bench;
#[cfg(feature = "client_requests_defs")]
pub mod client_requests;
#[cfg(feature = "default")]
mod common;
#[cfg(feature = "default")]
#[doc(hidden)]
pub mod internal;
#[cfg(feature = "default")]
mod lib_bench;
#[cfg(feature = "default")]
mod macros;

#[cfg(feature = "default")]
pub use bin_bench::{
    Arg, BenchmarkId, BinaryBenchmarkConfig, BinaryBenchmarkGroup, ExitWith, Fixtures, Run,
};
#[cfg(feature = "default")]
pub use bincode;
#[cfg(feature = "default")]
pub use common::{black_box, FlamegraphConfig, RegressionConfig, Tool};
#[cfg(feature = "client_requests_defs")]
pub use cty;
#[cfg(feature = "default")]
pub use iai_callgrind_macros::library_benchmark;
#[cfg(feature = "default")]
pub use iai_callgrind_runner::api::{Direction, EventKind, FlamegraphKind, ValgrindTool};
#[cfg(feature = "default")]
pub use lib_bench::LibraryBenchmarkConfig;
