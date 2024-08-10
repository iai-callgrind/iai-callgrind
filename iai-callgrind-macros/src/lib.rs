//! The library of iai-callgrind-macros

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(test(attr(warn(unused))))]
#![doc(test(attr(allow(unused_extern_crates))))]
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
#![allow(clippy::str_to_string)]

mod bin_bench;
mod common;
pub(crate) mod defaults;
mod derive_macros;
mod lib_bench;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    workspace_root: String,
}

/// The `#[library_benchmark]` attribute let's you define a benchmark function which you can later
/// use in the `library_benchmark_groups!` macro.
///
/// This attribute can be applied in two ways.
///
/// Using the `#[library_benchmark]` attribute as a standalone is fine for simple function calls
/// without parameters.
///
/// However, we mostly need to benchmark cases which would need to be setup for example with a
/// vector, but everything we setup within the benchmark function itself would be attributed to the
/// event counts. The second form of this attribute macro uses the `bench` attribute to setup
/// benchmarks with different cases. The main advantage is, that the setup costs and event counts
/// aren't attributed to the benchmark (and opposed to the old api we don't have to deal with
/// callgrind arguments, toggles, inline(never), ...)
///
/// The `bench` attribute consists of the attribute name itself, an unique id after `::` and
/// optionally one or more arguments with expressions which are passed to the benchmark function as
/// parameter, as shown below. The id has to be unique within `#[library_benchmark]` where it's
/// defined. However, the id can be the same as other id(s) in other `#[library_benchmark]` in the
/// same 'library_benchmark_group!` macro invocation and then they can be `used with
/// `library_benchmark_group!`'s optional parameter `compare_by_id`.
///
/// ```rust
/// # use iai_callgrind_macros::library_benchmark;
/// # mod iai_callgrind {
/// # pub struct LibraryBenchmarkConfig {}
/// # pub mod internal {
/// # pub struct InternalMacroLibBench {
/// #   pub id_display: Option<&'static str>,
/// #   pub args_display: Option<&'static str>,
/// #   pub func: fn(),
/// #   pub config: Option<fn() -> InternalLibraryBenchmarkConfig>
/// # }
/// # pub struct InternalLibraryBenchmarkConfig {}
/// # }
/// # }
/// // Assume this is a more complicated function in your library which you want to benchmark
/// fn some_func(value: u64) -> u64 {
///     42
/// }
///
/// #[library_benchmark]
/// #[bench::some_id(42)]
/// fn bench_some_func(value: u64) -> u64 {
///     std::hint::black_box(some_func(value))
/// }
/// # fn main() {}
/// ```
///
/// Assuming the same function `some_func`, the `benches` attribute lets you define multiple
/// benchmarks in one go:
/// ```rust
/// # use iai_callgrind_macros::library_benchmark;
/// # mod iai_callgrind {
/// # pub struct LibraryBenchmarkConfig {}
/// # pub mod internal {
/// # pub struct InternalMacroLibBench {
/// #   pub id_display: Option<&'static str>,
/// #   pub args_display: Option<&'static str>,
/// #   pub func: fn(),
/// #   pub config: Option<fn() -> InternalLibraryBenchmarkConfig>
/// # }
/// # pub struct InternalLibraryBenchmarkConfig {}
/// # }
/// # }
/// # fn some_func(value: u64) -> u64 {
/// #    value
/// # }
/// #[library_benchmark]
/// #[benches::some_id(21, 42, 84)]
/// fn bench_some_func(value: u64) -> u64 {
///     std::hint::black_box(some_func(value))
/// }
/// # fn main() {}
/// ```
///
/// # Examples
///
/// The `#[library_benchmark]` attribute as a standalone
///
/// ```rust
/// # use iai_callgrind_macros::library_benchmark;
/// # mod iai_callgrind {
/// # pub struct LibraryBenchmarkConfig {}
/// # pub mod internal {
/// # pub struct InternalMacroLibBench {
/// #   pub id_display: Option<&'static str>,
/// #   pub args_display: Option<&'static str>,
/// #   pub func: fn(),
/// #   pub config: Option<fn() -> InternalLibraryBenchmarkConfig>
/// # }
/// # pub struct InternalLibraryBenchmarkConfig {}
/// # }
/// # }
/// fn some_func() -> u64 {
///     42
/// }
///
/// #[library_benchmark]
/// // If possible, it's best to return something from a benchmark function
/// fn bench_my_library_function() -> u64 {
///     // The `black_box` is needed to tell the compiler to not optimize what's inside the
///     // black_box or else the benchmarks might return inaccurate results.
///     std::hint::black_box(some_func())
/// }
/// # fn main() {
/// # }
/// ```
///
///
/// In the following example we pass a single argument with `Vec<i32>` type to the benchmark. All
/// arguments are already wrapped in a black box and don't need to be put in a `black_box` again.
///
/// ```rust
/// # use iai_callgrind_macros::library_benchmark;
/// # mod iai_callgrind {
/// # pub struct LibraryBenchmarkConfig {}
/// # pub mod internal {
/// # pub struct InternalMacroLibBench {
/// #   pub id_display: Option<&'static str>,
/// #   pub args_display: Option<&'static str>,
/// #   pub func: fn(),
/// #   pub config: Option<fn() -> InternalLibraryBenchmarkConfig>
/// # }
/// # pub struct InternalLibraryBenchmarkConfig {}
/// # }
/// # }
/// // Our function we want to test. Just assume this is a public function in your
/// // library.
/// fn some_func_with_array(array: Vec<i32>) -> Vec<i32> {
///     // do something with the array and return a new array
///     # array
/// }
///
/// // This function is used to create a worst case array for our `some_func_with_array`
/// fn setup_worst_case_array(start: i32) -> Vec<i32> {
///     if start.is_negative() {
///         (start..0).rev().collect()
///     } else {
///         (0..start).rev().collect()
///     }
/// }
///
/// // This benchmark is setting up multiple benchmark cases with the advantage that the setup
/// // costs  for creating a vector (even if it is empty) aren't attributed to the benchmark and
/// // that the `array` is already wrapped in a black_box.
/// #[library_benchmark]
/// #[bench::empty(vec![])]
/// #[bench::worst_case_6(vec![6, 5, 4, 3, 2, 1])]
/// // Function calls are fine too
/// #[bench::worst_case_4000(setup_worst_case_array(4000))]
/// // The argument of the benchmark function defines the type of the argument from the `bench`
/// // cases.
/// fn bench_some_func_with_array(array: Vec<i32>) -> Vec<i32> {
///     // Note `array` does not need to be put in a `black_box` because that's already done for
///     // you.
///     std::hint::black_box(some_func_with_array(array))
/// }
///
/// // The following benchmark uses the `#[benches]` attribute to setup multiple benchmark cases
/// // in one go
/// #[library_benchmark]
/// #[benches::multiple(vec![1], vec![5])]
/// // Reroute the `args` to a `setup` function and use the setup function's return value as
/// // input for the benchmarking function
/// #[benches::with_setup(args = [1, 5], setup = setup_worst_case_array)]
/// fn bench_using_the_benches_attribute(array: Vec<i32>) -> Vec<i32> {
///     std::hint::black_box(some_func_with_array(array))
/// }
/// # fn main() {
/// # }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn library_benchmark(args: TokenStream, input: TokenStream) -> TokenStream {
    match lib_bench::render(args.into(), input.into()) {
        Ok(stream) => stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// TODO: DOCUMENTATION
#[proc_macro_attribute]
#[proc_macro_error]
pub fn binary_benchmark(args: TokenStream, input: TokenStream) -> TokenStream {
    match bin_bench::render(args.into(), input.into()) {
        Ok(stream) => stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// For internal use only.
///
/// The old `macro_rules! impl_traits` was easy to overlook in the source code files and this derive
/// macro is just a much nicer way to do the same.
///
/// We use this derive macro to spare us the manual implementation of
///
/// * From<Outer> for Inner
/// * From<&Outer> for Inner (which clones the value)
/// * From<&mut Outer> for Inner (which also just clones the value)
///
/// for our builder tuple structs which wrap the inner type from the iai-callgrind-runner api. So,
/// our builders don't need a build method, which is just cool.
#[proc_macro_derive(IntoInner)]
#[proc_macro_error]
pub fn into_inner(item: TokenStream) -> TokenStream {
    match derive_macros::render_into_inner(item.into()) {
        Ok(stream) => stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
