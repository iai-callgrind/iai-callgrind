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
use proc_macro_error::{abort, emit_error, proc_macro_error};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{
    parse2, parse_quote, Attribute, Expr, ExprArray, ExprPath, Ident, ItemFn, MetaList,
    MetaNameValue, Token,
};

#[derive(Debug, Clone)]
struct Arguments(Vec<Expr>);

#[derive(Debug)]
struct LibBenchAttribute {
    id: Ident,
    args: Arguments,
    config: Option<Expr>,
    setup: Option<ExprPath>,
}

#[derive(Debug, Default)]
struct LibraryBenchmark {
    config: Option<Expr>,
    benches: Vec<LibBenchAttribute>,
}

#[derive(Debug, Clone)]
struct MultipleArguments(Vec<Arguments>);

impl Arguments {
    fn len(&self) -> usize {
        self.0.len()
    }
}

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
            #(std::hint::black_box(#exprs)),*
        };
        tokens.append_all(this_tokens);
    }
}

impl LibBenchAttribute {
    fn render_as_function(&self, callee: &Ident) -> TokenStream2 {
        let id = &self.id;

        let func = if let Some(setup) = &self.setup {
            let exprs = &self.args.0;
            let args = quote! {
                #(#exprs),*
            };
            quote! {
                #[inline(never)]
                pub fn #id() {
                    let _ = std::hint::black_box(#callee(std::hint::black_box(#setup(#args))));
                }
            }
        } else {
            let args = &self.args;
            quote! {
                #[inline(never)]
                pub fn #id() {
                    let _ = std::hint::black_box(#callee(#args));
                }
            }
        };
        if let Some(config) = &self.config {
            let config_ident = format_ident!("get_config_{}", id);
            quote! {
                #[inline(never)]
                pub fn #config_ident() -> iai_callgrind::internal::InternalLibraryBenchmarkConfig {
                    #config.into()
                }

                #func
            }
        } else {
            func
        }
    }

    fn render_as_lib_bench(&self) -> TokenStream2 {
        let id = &self.id;
        let id_str = self.id.to_string();
        let args = if let Some(setup) = self.setup.as_ref() {
            let exprs = &self.args.0;
            let tokens = quote! {
                #(#exprs),*
            };
            quote! { #setup(#tokens) }.to_string()
        } else {
            self.args
                .0
                .iter()
                .map(|a| a.to_token_stream().to_string())
                .collect::<Vec<String>>()
                .join(", ")
        };
        if self.config.is_some() {
            let conf_ident = format_ident!("get_config_{}", id);
            quote! {
                iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some(#id_str),
                    args_display: Some(#args),
                    func: #id,
                    config: Some(#conf_ident)
                }
            }
        } else {
            quote! {
                iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some(#id_str),
                    args_display: Some(#args),
                    func: #id,
                    config: None
                }
            }
        }
    }
}

impl LibraryBenchmark {
    #[allow(clippy::too_many_lines)]
    fn parse_bench_attribute(
        &mut self,
        item_fn: &ItemFn,
        attr: &Attribute,
        id: Ident,
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;
        if let Ok(pairs) =
            meta.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        {
            let mut args = None;
            let mut config = None;
            for pair in pairs {
                if pair.path.segments.is_empty() {
                    emit_error!(
                        pair, "Missing key";
                        help = "At least one argument must be given";
                        note = "Valid arguments are: args, config"
                    );
                } else if pair.path.is_ident("args") {
                    if args.is_none() {
                        args = Some(pair.value);
                    } else {
                        emit_error!(
                            pair, "Duplicate argument: args";
                            help = "args is allowed only once"
                        );
                    }
                } else if pair.path.is_ident("config") {
                    if config.is_none() {
                        config = Some(pair.value);
                    } else {
                        emit_error!(
                            pair, "Duplicate argument: config";
                            help = "config is allowed only once"
                        );
                    }
                } else {
                    abort!(
                        pair, "Invalid argument: {}", pair.path.get_ident().unwrap();
                        help = "Valid arguments are: args, config"
                    );
                }
            }
            if let Some(expr) = args {
                let args = match expr {
                    Expr::Array(items) => parse2::<Arguments>(items.elems.to_token_stream())?,
                    Expr::Tuple(items) => parse2::<Arguments>(items.elems.to_token_stream())?,
                    Expr::Paren(item) if expected_num_args == 1 => Arguments(vec![*item.expr]),
                    _ => {
                        abort!(
                            expr,
                            "Failed parsing `args`";
                            help = "`args` must be an tuple/array which elements (expressions)
                            match the number of parameters of the benchmarking function";
                            note = "#[bench::id(args = (1, 2))] or
                            #[bench::id(args = [1, 2]])]"
                        );
                    }
                };
                if args.len() == expected_num_args {
                    self.benches.push(LibBenchAttribute {
                        id,
                        args,
                        config,
                        setup: None,
                    });
                } else {
                    emit_error!(
                        meta,
                        "Expected {} arguments but found {}",
                        expected_num_args,
                        args.len()
                    );
                };
            } else if expected_num_args == 0 {
                self.benches.push(LibBenchAttribute {
                    id,
                    args: Arguments(vec![]),
                    config,
                    setup: None,
                });
            } else {
                emit_error!(
                    attr, "Expected {} arguments but found none", expected_num_args;
                    help = "Try passing arguments either with #[bench::some_id(arg1, ...)]
                or with #[bench::some_id(args = (arg1, ...))]"
                );
            }
        } else {
            let args = meta.parse_args::<Arguments>()?;
            if args.len() == expected_num_args {
                self.benches.push(LibBenchAttribute {
                    id,
                    args,
                    config: None,
                    setup: None,
                });
            } else {
                emit_error!(
                    meta,
                    "Expected {} arguments but found {}",
                    expected_num_args,
                    args.len()
                );
            }
        }
        Ok(())
    }

    fn parse_benches_attribute(
        &mut self,
        item_fn: &ItemFn,
        attr: &Attribute,
        id: &Ident,
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut config = None;
        let mut setup = None;
        let args = if let Ok(pairs) =
            meta.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        {
            let mut args = None;
            for pair in pairs {
                if pair.path.segments.is_empty() {
                    abort!(
                        pair, "Missing key";
                        help = "At least one argument must be given";
                        note = "Valid arguments are: `args`, `config`, `setup`"
                    );
                } else if pair.path.is_ident("args") {
                    if args.is_none() {
                        args = Some(pair.value);
                    } else {
                        abort!(
                            pair, "Duplicate argument: `args`";
                            help = "`args` is allowed only once"
                        );
                    }
                } else if pair.path.is_ident("config") {
                    if config.is_none() {
                        config = Some(pair.value);
                    } else {
                        abort!(
                            pair, "Duplicate argument: `config`";
                            help = "`config` is allowed only once"
                        );
                    }
                } else if pair.path.is_ident("setup") {
                    if setup.is_none() {
                        if let Expr::Path(path) = pair.value {
                            setup = Some(path);
                        } else {
                            abort!(
                                pair, "Invalid value for `setup`";
                                help = "The `setup` argument needs a path to an existing function
                            in a reachable scope";
                                note = "`setup = my_setup` or `setup = my::setup::function`"
                            );
                        }
                    } else {
                        abort!(
                            pair, "Duplicate argument: `setup`";
                            help = "`setup` is allowed only once"
                        );
                    }
                } else {
                    abort!(
                        pair, "Invalid argument: {}", pair.path.get_ident().unwrap();
                        help = "Valid arguments are: `args`, `config`, `setup`"
                    );
                }
            }
            if let Some(args) = args {
                MultipleArguments::from_expr(&args, expected_num_args, setup.is_some())?
            } else if setup.is_none() && expected_num_args != 0 {
                abort!(
                    meta, "Missing arguments for `benches`";
                    help = "Either specify the `args` argument or use plain arguments";
                    note = "`#[benches::id(args = [...])]` or `#[benches::id(1, 2, ...)]`"
                );
            } else {
                MultipleArguments(vec![])
            }
        } else {
            MultipleArguments::from_meta_list(meta, expected_num_args)?
        };

        for (i, args) in args.0.into_iter().enumerate() {
            let id = format_ident!("{id}_{i}");
            if (setup.is_none() && args.len() == expected_num_args) || setup.is_some() {
                self.benches.push(LibBenchAttribute {
                    id,
                    args,
                    config: config.clone(),
                    setup: setup.clone(),
                });
            } else {
                emit_error!(
                    meta,
                    "Expected {} arguments but found {}",
                    expected_num_args,
                    args.len()
                );
            }
        }
        Ok(())
    }

    fn extract_benches(&mut self, item_fn: &ItemFn) -> syn::Result<()> {
        let bench: syn::PathSegment = parse_quote!(bench);
        let benches: syn::PathSegment = parse_quote!(benches);
        for attr in &item_fn.attrs {
            let mut path_segments = attr.path().segments.iter();
            match path_segments.next() {
                Some(segment) if segment == &bench => {
                    if attr.path().segments.len() > 2 {
                        abort!(
                            attr, "Only one id is allowed";
                            help = "bench followed by :: and an single unique id";
                            note = r#"#[bench::my_id()] or #[bench::my_id("with", "args")]
                        or #[bench::my_id(args = (arg1, ...), config = ...)]"#
                        );
                    }
                    let Some(id) = path_segments.next().map(|p| p.ident.clone()) else {
                        abort!(
                            attr, "An id is required";
                            help = "bench followed by :: and an unique id";
                            note = "#[bench::my_id(...)]"
                        );
                    };
                    self.parse_bench_attribute(item_fn, attr, id)?;
                }
                Some(segment) if segment == &benches => {
                    if attr.path().segments.len() > 2 {
                        abort!(
                            attr, "Only one id is allowed";
                            help = "benches followed by :: and an single unique id";
                            note = r#"#[benches::my_id("with", "args")]
                        or #[benches::my_id(args = [arg1, ...]]"#
                        );
                    }
                    let Some(id) = path_segments.next().map(|p| p.ident.clone()) else {
                        abort!(
                            attr, "An id is required";
                            help = "benches followed by :: and an unique id";
                            note = "#[benches::my_id(...)]"
                        );
                    };
                    self.parse_benches_attribute(item_fn, attr, &id)?;
                }
                Some(segment) => {
                    abort!(
                        attr, "Invalid attribute: '{}'", segment.ident;
                        help = "Only the `bench` and the `benches` attribute are allowed";
                        note = r#"#[bench::my_id("with", "args")]
                    or #[benches::my_id(args = [("with", "args"), ...)])]"#
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

    fn render_single(self, item_fn: &ItemFn) -> TokenStream2 {
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let ident = &item_fn.sig.ident;
        let export_name = format!("iai_callgrind::bench::{}", &item_fn.sig.ident);
        let config = if let Some(config) = self.config {
            quote!(
                #[inline(never)]
                pub fn get_config()
                    -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    Some(#config.into())
                }
            )
        } else {
            quote!(
                #[inline(never)]
                pub fn get_config(
                ) -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    None
                }
            )
        };
        quote! {
            mod #ident {
                use super::*;

                #[inline(never)]
                #[export_name = #export_name]
                #new_item_fn

                pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench]= &[
                    iai_callgrind::internal::InternalMacroLibBench {
                        id_display: None,
                        args_display: None,
                        func: wrapper,
                        config: None
                    },
                ];

                #config

                #[inline(never)]
                pub fn wrapper() {
                    let _ = std::hint::black_box(#ident());
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
        let export_name = format!("iai_callgrind::bench::{}", &item_fn.sig.ident);
        let callee = &item_fn.sig.ident;
        let mut funcs = TokenStream2::new();
        let mut lib_benches = vec![];
        for bench in self.benches {
            funcs.append_all(bench.render_as_function(callee));
            lib_benches.push(bench.render_as_lib_bench());
        }

        let config = if let Some(config) = self.config {
            quote!(
                #[inline(never)]
                pub fn get_config()
                -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    Some(#config.into())
                }
            )
        } else {
            quote!(
                #[inline(never)]
                pub fn get_config(
                ) -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    None
                }
            )
        };
        quote! {
            mod #mod_name {
                use super::*;

                #[inline(never)]
                #[export_name = #export_name]
                #new_item_fn

                pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench]= &[
                    #(#lib_benches,)*
                ];

                #config

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
            let pairs = input.parse_terminated(MetaNameValue::parse, Token![,])?;
            if pairs.len() > 1 {
                abort!(
                    pairs, "At most one argument is allowed";
                    help = "#[library_benchmark] or #[library_benchmark(config = ....)]"
                );
            } else {
                Ok(pairs.first().map_or_else(Self::default, |pair| {
                    if pair.path.is_ident("config") {
                        Self {
                            config: Some(pair.value.clone()),
                            ..Default::default()
                        }
                    } else {
                        abort!(
                            pair, "Only the config argument is allowed";
                            help = "#[library_benchmark(config = ....)]"
                        );
                    }
                }))
            }
        }
    }
}

impl MultipleArguments {
    fn from_expr(expr: &Expr, expected_num_args: usize, has_setup: bool) -> syn::Result<Self> {
        let expr_array = parse2::<ExprArray>(expr.to_token_stream())?;
        let mut values: Vec<Arguments> = vec![];
        for elem in expr_array.elems {
            match elem {
                Expr::Tuple(items) => {
                    values.push(parse2(items.elems.to_token_stream())?);
                }
                Expr::Paren(item) if has_setup || expected_num_args == 1 => {
                    values.push(Arguments(vec![*item.expr]));
                }
                _ if has_setup || expected_num_args == 1 => {
                    values.push(Arguments(vec![elem]));
                }
                _ => {
                    abort!(
                        elem,
                        "Failed parsing arguments: Expected {} values per tuple",
                        expected_num_args;
                        help = "If the benchmarking function has multiple parameters
                    the arguments for #[benches::...] must be given as tuple";
                        note = "#[benches::id((1, 2), (3, 4))] or \
                               #[benches::id(args = [(1, 2), (3, 4)])]";
                    );
                }
            }
        }
        Ok(Self(values))
    }

    fn from_meta_list(meta: &MetaList, expected_num_args: usize) -> syn::Result<Self> {
        let list = &meta.tokens;
        let expr = parse2::<Expr>(quote! { [#list] })?;
        Self::from_expr(&expr, expected_num_args, false)
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
        Ok(library_benchmark.render_single(&item_fn))
    } else {
        Ok(library_benchmark.render_benches(&item_fn))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use syn::{ExprStruct, ItemMod};

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Model {
        item: ItemMod,
    }

    impl Parse for Model {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            Ok(Self {
                item: input.parse::<ItemMod>()?,
            })
        }
    }

    fn expected_model(
        func: &ItemFn,
        benches: &[ExprStruct],
        get_config: &Option<Expr>,
        get_config_bench: &[(Ident, Expr)],
        bench: &[(Ident, Vec<Expr>)],
    ) -> Model {
        let callee = &func.sig.ident;
        let export_name = format!("iai_callgrind::bench::{}", &func.sig.ident);
        let rendered_get_config = if let Some(expr) = get_config {
            quote!(
                #[inline(never)]
                pub fn get_config()
                -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    Some(#expr.into())
                }
            )
        } else {
            quote!(
                #[inline(never)]
                pub fn get_config(
                ) -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig>
                {
                    None
                }
            )
        };
        let mut rendered_benches = vec![];
        for (ident, args) in bench {
            let config = get_config_bench.iter().find_map(|(i, expr)| {
                (i == ident).then(|| {
                    let ident = format_ident!("get_config_{}", i);
                    quote!(
                        #[inline(never)]
                        pub fn #ident() -> iai_callgrind::internal::InternalLibraryBenchmarkConfig {
                            #expr.into()
                        }
                    )
                })
            });
            if let Some(config) = config {
                rendered_benches.push(config);
            }
            rendered_benches.push(quote!(
                #[inline(never)]
                pub fn #ident() {
                    let _ = std::hint::black_box(#callee(
                        #(std::hint::black_box(#args)),*
                    ));
                }
            ));
        }
        parse_quote!(
            mod #callee {
                use super::*;

                #[inline(never)]
                #[export_name = #export_name]
                #func

                pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench]= &[
                    #(#benches),*,
                ];

                #rendered_get_config

                #(#rendered_benches)*
            }
        )
    }

    #[test]
    fn test_only_library_benchmark_attribute() {
        let input = quote!(
            fn some() -> u8 {
                1 + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some() -> u8 {
                    1 + 2
                }
            ),
            &[parse_quote!(
                iai_callgrind::internal::InternalMacroLibBench {
                    id_display: None,
                    args_display: None,
                    func: wrapper,
                    config: None
                }
            )],
            &None,
            &[],
            &[(parse_quote!(wrapper), vec![])],
        );
        let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_only_library_benchmark_attribute_with_config() {
        let input = quote!(
            fn some() -> u8 {
                1 + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some() -> u8 {
                    1 + 2
                }
            ),
            &[parse_quote!(
                iai_callgrind::internal::InternalMacroLibBench {
                    id_display: None,
                    args_display: None,
                    func: wrapper,
                    config: None
                }
            )],
            &Some(parse_quote!(LibraryBenchmarkConfig::default())),
            &[],
            &[(parse_quote!(wrapper), vec![])],
        );
        let actual: Model = parse2(
            render_library_benchmark(quote!(config = LibraryBenchmarkConfig::default()), input)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_func_no_arg() {
        for attribute in [
            quote!(bench::my_id()),
            quote!(bench::my_id(args = ())),
            quote!(bench::my_id(args = [])),
        ] {
            dbg!(&attribute);
            let input = quote!(
                #[#attribute]
                fn some() -> u8 {
                    1 + 2
                }
            );

            let expected = expected_model(
                &parse_quote!(
                    fn some() -> u8 {
                        1 + 2
                    }
                ),
                &[parse_quote!(
                    iai_callgrind::internal::InternalMacroLibBench {
                        id_display: Some("my_id"),
                        args_display: Some(""),
                        func: my_id,
                        config: None
                    }
                )],
                &None,
                &[],
                &[(parse_quote!(my_id), vec![])],
            );
            let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_bench_when_func_one_arg() {
        for attribute in [
            quote!(bench::my_id(1)),
            quote!(bench::my_id(args = (1,))),
            quote!(bench::my_id(args = (1))),
            quote!(bench::my_id(args = [1])),
        ] {
            dbg!(&attribute);
            let input = quote!(
                #[#attribute]
                fn some(var: u8) -> u8 {
                    var + 2
                }
            );

            let expected = expected_model(
                &parse_quote!(
                    fn some(var: u8) -> u8 {
                        var + 2
                    }
                ),
                &[parse_quote!(
                    iai_callgrind::internal::InternalMacroLibBench {
                        id_display: Some("my_id"),
                        args_display: Some("1"),
                        func: my_id,
                        config: None
                    }
                )],
                &None,
                &[],
                &[(parse_quote!(my_id), vec![parse_quote!(1)])],
            );
            let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_bench_when_func_two_args() {
        for attribute in [
            quote!(bench::my_id(1, 2)),
            quote!(bench::my_id(args = (1, 2))),
            quote!(bench::my_id(args = [1, 2])),
        ] {
            dbg!(&attribute);
            let input = quote!(
                #[#attribute]
                fn some(one: u8, two: u8) -> u8 {
                    one + two
                }
            );

            let expected = expected_model(
                &parse_quote!(
                    fn some(one: u8, two: u8) -> u8 {
                        one + two
                    }
                ),
                &[parse_quote!(
                    iai_callgrind::internal::InternalMacroLibBench {
                        id_display: Some("my_id"),
                        args_display: Some("1, 2"),
                        func: my_id,
                        config: None
                    }
                )],
                &None,
                &[],
                &[(parse_quote!(my_id), vec![parse_quote!(1), parse_quote!(2)])],
            );
            let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_bench_when_config_no_args() {
        for attribute in [
            quote!(bench::my_id(config = LibraryBenchmarkConfig::default())),
            quote!(bench::my_id(
                args = (),
                config = LibraryBenchmarkConfig::default()
            )),
        ] {
            dbg!(&attribute);
            let input = quote!(
                #[#attribute]
                fn some() -> u8 {
                    1 + 2
                }
            );

            let expected = expected_model(
                &parse_quote!(
                    fn some() -> u8 {
                        1 + 2
                    }
                ),
                &[parse_quote!(
                    iai_callgrind::internal::InternalMacroLibBench {
                        id_display: Some("my_id"),
                        args_display: Some(""),
                        func: my_id,
                        config: Some(get_config_my_id)
                    }
                )],
                &None,
                &[(
                    parse_quote!(my_id),
                    parse_quote!(LibraryBenchmarkConfig::default()),
                )],
                &[(parse_quote!(my_id), vec![])],
            );
            let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_bench_when_config_and_library_benchmark_config() {
        let attribute = quote!(bench::my_id(config = LibraryBenchmarkConfig::default()));
        dbg!(&attribute);
        let input = quote!(
            #[#attribute]
            fn some() -> u8 {
                1 + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some() -> u8 {
                    1 + 2
                }
            ),
            &[parse_quote!(
                iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("my_id"),
                    args_display: Some(""),
                    func: my_id,
                    config: Some(get_config_my_id)
                }
            )],
            &Some(parse_quote!(LibraryBenchmarkConfig::new())),
            &[(
                parse_quote!(my_id),
                parse_quote!(LibraryBenchmarkConfig::default()),
            )],
            &[(parse_quote!(my_id), vec![])],
        );
        let actual: Model = parse2(
            render_library_benchmark(quote!(config = LibraryBenchmarkConfig::new()), input)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_multiple_no_args() {
        let input = quote!(
            #[bench::first()]
            #[bench::second()]
            fn some() -> u8 {
                1 + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some() -> u8 {
                    1 + 2
                }
            ),
            &[
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("first"),
                    args_display: Some(""),
                    func: first,
                    config: None
                }),
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("second"),
                    args_display: Some(""),
                    func: second,
                    config: None
                }),
            ],
            &None,
            &[],
            &[
                (parse_quote!(first), vec![]),
                (parse_quote!(second), vec![]),
            ],
        );
        let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_multiple_one_arg() {
        let input = quote!(
            #[bench::first(1)]
            #[bench::second(2)]
            fn some(var: u8) -> u8 {
                var + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some(var: u8) -> u8 {
                    var + 2
                }
            ),
            &[
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("first"),
                    args_display: Some("1"),
                    func: first,
                    config: None
                }),
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("second"),
                    args_display: Some("2"),
                    func: second,
                    config: None
                }),
            ],
            &None,
            &[],
            &[
                (parse_quote!(first), vec![parse_quote!(1)]),
                (parse_quote!(second), vec![parse_quote!(2)]),
            ],
        );
        let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_multiple_with_config_first() {
        let input = quote!(
            #[bench::first(args = (1), config = LibraryBenchmarkConfig::default())]
            #[bench::second(2)]
            fn some(var: u8) -> u8 {
                var + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some(var: u8) -> u8 {
                    var + 2
                }
            ),
            &[
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("first"),
                    args_display: Some("1"),
                    func: first,
                    config: Some(get_config_first)
                }),
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("second"),
                    args_display: Some("2"),
                    func: second,
                    config: None
                }),
            ],
            &None,
            &[(
                parse_quote!(first),
                parse_quote!(LibraryBenchmarkConfig::default()),
            )],
            &[
                (parse_quote!(first), vec![parse_quote!(1)]),
                (parse_quote!(second), vec![parse_quote!(2)]),
            ],
        );
        let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_multiple_with_config_second() {
        let input = quote!(
            #[bench::first(1)]
            #[bench::second(args = (2), config = LibraryBenchmarkConfig::default())]
            fn some(var: u8) -> u8 {
                var + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some(var: u8) -> u8 {
                    var + 2
                }
            ),
            &[
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("first"),
                    args_display: Some("1"),
                    func: first,
                    config: None
                }),
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("second"),
                    args_display: Some("2"),
                    func: second,
                    config: Some(get_config_second)
                }),
            ],
            &None,
            &[(
                parse_quote!(second),
                parse_quote!(LibraryBenchmarkConfig::default()),
            )],
            &[
                (parse_quote!(first), vec![parse_quote!(1)]),
                (parse_quote!(second), vec![parse_quote!(2)]),
            ],
        );
        let actual: Model = parse2(render_library_benchmark(quote!(), input).unwrap()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bench_when_multiple_with_config_all() {
        let input = quote!(
            #[bench::first(args = (1), config = LibraryBenchmarkConfig::new())]
            #[bench::second(args = (2), config = LibraryBenchmarkConfig::default())]
            fn some(var: u8) -> u8 {
                var + 2
            }
        );

        let expected = expected_model(
            &parse_quote!(
                fn some(var: u8) -> u8 {
                    var + 2
                }
            ),
            &[
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("first"),
                    args_display: Some("1"),
                    func: first,
                    config: Some(get_config_first)
                }),
                parse_quote!(iai_callgrind::internal::InternalMacroLibBench {
                    id_display: Some("second"),
                    args_display: Some("2"),
                    func: second,
                    config: Some(get_config_second)
                }),
            ],
            &Some(parse_quote!(LibraryBenchmarkConfig::does_not_exist())),
            &[
                (
                    parse_quote!(first),
                    parse_quote!(LibraryBenchmarkConfig::new()),
                ),
                (
                    parse_quote!(second),
                    parse_quote!(LibraryBenchmarkConfig::default()),
                ),
            ],
            &[
                (parse_quote!(first), vec![parse_quote!(1)]),
                (parse_quote!(second), vec![parse_quote!(2)]),
            ],
        );

        let actual: Model = parse2(
            render_library_benchmark(
                quote!(config = LibraryBenchmarkConfig::does_not_exist()),
                input,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
