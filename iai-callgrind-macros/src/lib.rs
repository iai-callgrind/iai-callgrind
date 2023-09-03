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

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::Parse;
use syn::{parse2, parse_quote, Expr, Ident, ItemFn, Token};

#[derive(Debug, Default)]
struct LibraryBenchmark {
    benches: Vec<Bench>,
}

impl LibraryBenchmark {
    fn extract_benches(&mut self, item_fn: &ItemFn) -> syn::Result<()> {
        let bench: syn::PathSegment = parse_quote!(bench);
        for attr in &item_fn.attrs {
            let mut path_segments = attr.path().segments.iter();
            match path_segments.next() {
                Some(segment) if segment == &bench => {
                    if attr.path().segments.len() > 2 {
                        abort!(
                            attr, "Only one id is allowed";
                            help = "bench followed by :: and an single unique id";
                            note = r#"#[bench::my_id()] or #[bench::my_id("with", "args")]"#
                        );
                    }
                    let id = match path_segments.next().map(|p| p.ident.clone()) {
                        Some(id) => id,
                        None => {
                            abort!(
                                attr, "An id is required";
                                help = "bench followed by :: and an unique id";
                                note = "bench::my_id"
                            );
                        }
                    };
                    let args = attr.parse_args::<Arguments>()?;
                    self.benches.push(Bench { id, args });
                }
                Some(segment) => {
                    abort!(
                        attr, "Invalid attribute: '{}'", segment.ident;
                        help = "Only the 'bench' attribute is allowed";
                        note = r#"#[bench::my_id()] or #[bench::my_id("with", "args")]"#
                    );
                }
                None => {
                    // #[] => Syntax error: Expected an identifier
                    unreachable!("This case is handled by the compiler")
                }
            }
        }

        Ok(())
    }

    fn render_single(item_fn: &ItemFn) -> TokenStream2 {
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let ident = &item_fn.sig.ident;
        quote! {
            mod #ident {
                use super::*;

                #[inline(never)]
                #new_item_fn

                pub const FUNCTIONS: &[&(&'static str, &'static str, fn())]= &[
                    &("", "", wrapper),
                ];

                #[inline(never)]
                fn wrapper() {
                    let _ = iai_callgrind::black_box(#ident());
                }
            }
        }
    }

    fn render_benches(self, item_fn: &ItemFn) -> TokenStream2 {
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let mod_name = &item_fn.sig.ident;
        let callee = &item_fn.sig.ident;
        let mut funcs = TokenStream2::new();
        let mut args = vec![];
        for bench in self.benches {
            funcs.append_all(bench.render_as_function(callee));
            args.push(bench.render_as_arguments());
        }

        quote! {
            mod #mod_name {
                use super::*;

                #[inline(never)]
                #new_item_fn

                pub const FUNCTIONS: &[&(&'static str, &'static str, fn())]= &[
                    #(&(#args),)*
                ];

                #funcs
            }
        }
    }
}

impl Parse for LibraryBenchmark {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self::default())
        } else {
            Err(input.error("No arguments allowed"))
        }
    }
}

#[derive(Debug)]
struct Bench {
    id: Ident,
    args: Arguments,
}

impl Bench {
    fn render_as_function(&self, callee: &Ident) -> TokenStream2 {
        let id = &self.id;
        let args = &self.args;

        quote! {
            #[inline(never)]
            pub fn #id() {
                let _ = iai_callgrind::black_box(#callee(#args));
            }
        }
    }

    fn render_as_arguments(&self) -> TokenStream2 {
        let id = &self.id;
        let id_str = self.id.to_string();
        let args = self
            .args
            .0
            .iter()
            .map(|a| a.to_token_stream().to_string())
            .collect::<Vec<String>>()
            .join(", ");
        quote! {
            #id_str, #args, #id
        }
    }
}

#[derive(Debug, Clone)]
struct Arguments(Vec<Expr>);

impl Parse for Arguments {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let data = input
            .parse_terminated(Parse::parse, Token![,])?
            .into_iter()
            .collect();
        Ok(Self(data))
    }
}

impl ToTokens for Arguments {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let exprs = &self.0;
        let this_tokens = quote! {
            #(iai_callgrind::black_box(#exprs)),*
        };
        tokens.append_all(this_tokens);
    }
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
/// The `bench` attribute consist of the attribute name itself, an unique id after `::` and
/// optionally one or more arguments with expressions which are passed to the benchmark function as
/// parameter as shown below:
///
/// ```rust
/// # use iai_callgrind_macros::library_benchmark;
/// # fn black_box<T>(arg: T) -> T {
/// # arg
/// # }
/// # mod iai_callgrind {
/// # pub fn black_box<T>(arg: T) -> T {
/// # arg
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
///     black_box(some_func(value))
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
/// # fn black_box<T>(arg: T) -> T {
/// # arg
/// # }
/// # mod iai_callgrind {
/// # pub fn black_box<T>(arg: T) -> T {
/// # arg
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
///     black_box(some_func())
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
/// # fn black_box<T>(arg: T) -> T {
/// # arg
/// # }
/// # mod iai_callgrind {
/// # pub fn black_box<T>(arg: T) -> T {
/// # arg
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
/// // This benchmark is setting up multiple benchmark cases with the advantage that the setup costs
/// // for creating a vector (even if it is empty) aren't attributed to the benchmark and that the
/// // `array` is already wrapped in a black_box.
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
///     black_box(some_func_with_array(array))
/// }
/// # fn main() {
/// # }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn library_benchmark(args: TokenStream, input: TokenStream) -> TokenStream {
    match render_library_benchmark(args.into(), input.into()) {
        Ok(stream) => stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn render_library_benchmark(args: TokenStream2, input: TokenStream2) -> syn::Result<TokenStream2> {
    let mut library_benchmark = parse2::<LibraryBenchmark>(args)?;
    let item_fn = parse2::<ItemFn>(input)?;

    library_benchmark.extract_benches(&item_fn)?;
    if library_benchmark.benches.is_empty() {
        Ok(LibraryBenchmark::render_single(&item_fn))
    } else {
        Ok(library_benchmark.render_benches(&item_fn))
    }
}
