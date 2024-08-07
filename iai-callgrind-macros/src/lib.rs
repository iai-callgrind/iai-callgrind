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

use std::fs::File as StdFile;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

// TODO: CLEARIFY USAGE OF TokenStream vs TokenStream2
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_error::{abort, emit_error, proc_macro_error};
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use serde::Deserialize;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse2, parse_quote, parse_quote_spanned, Attribute, Expr, ExprArray, ExprPath, Ident, ItemFn,
    LitStr, MetaList, MetaNameValue, Token,
};

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    workspace_root: String,
}

/// This struct reflects the `args` parameter of the `#[bench]` attribute
#[derive(Debug, Default, Clone)]
struct Args(Option<(Span, Vec<Expr>)>);

/// This is the counterpart for the `#[bench]` attribute
///
/// The #[benches] attribute is also parsed into this structure.
#[derive(Debug)]
struct Bench {
    id: Ident,
    args: Args,
    config: BenchConfig,
    setup: Setup,
    teardown: Teardown,
}

/// The `config` parameter of the `#[bench]` or `#[benches]` attribute
#[derive(Debug, Default, Clone)]
struct BenchConfig(Option<Expr>);

/// This is the counterpart to the `#[library_benchmark]` attribute.
#[derive(Debug, Default)]
struct LibraryBenchmark {
    config: LibraryBenchmarkConfig,
    setup: Setup,
    teardown: Teardown,
    benches: Vec<Bench>,
}

/// The `config` parameter of the `#[library_benchmark]` attribute
///
/// The `BenchConfig` and `LibraryBenchmarkConfig` are rendered differently, hence the different
/// structures
///
/// Note: This struct is completely independent of the `iai_callgrind::LibraryBenchmarkConfig`
/// struct with the same name.
#[derive(Debug, Default, Clone)]
struct LibraryBenchmarkConfig(Option<Expr>);

/// This struct stores multiple `Args` as needed by the `#[benches]` attribute
#[derive(Debug, Clone, Default)]
struct MultipleArgs(Option<Vec<Args>>);

#[derive(Debug, Default, Clone)]
struct Setup(Option<ExprPath>);

#[derive(Debug, Default, Clone)]
struct Teardown(Option<ExprPath>);

#[derive(Debug, Default, Clone)]
struct File(Option<LitStr>);

impl Args {
    fn new(span: Span, data: Vec<Expr>) -> Self {
        Self(Some((span, data)))
    }

    fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |(_, data)| data.len())
    }

    fn span(&self) -> Option<&Span> {
        self.0.as_ref().map(|(span, _)| span)
    }

    fn set_span(&mut self, span: Span) {
        if let Some(data) = self.0.as_mut() {
            data.0 = span;
        }
    }

    fn parse_pair(&mut self, pair: &MetaNameValue) -> syn::Result<()> {
        if self.0.is_none() {
            let expr = &pair.value;
            let span = expr.span();
            let args = match expr {
                Expr::Array(items) => {
                    let mut args = parse2::<Args>(items.elems.to_token_stream())?;
                    // Set span explicitly (again) to overwrite the wrong span from parse2
                    args.set_span(span);
                    args
                }
                Expr::Tuple(items) => {
                    let mut args = parse2::<Args>(items.elems.to_token_stream())?;
                    // Set span explicitly (again) to overwrite the wrong span from parse2
                    args.set_span(span);
                    args
                }
                Expr::Paren(item) => Self::new(span, vec![(*item.expr).clone()]),
                _ => {
                    abort!(
                        expr,
                        "Failed parsing `args`";
                        help = "`args` has to be a tuple/array which elements (expressions)
                        match the number of parameters of the benchmarking function";
                        note = "#[bench::id(args = (1, 2))] or
                        #[bench::id(args = [1, 2]])]"
                    );
                }
            };

            *self = args;
        } else {
            emit_error!(
                pair, "Duplicate argument: `args`";
                help = "`args` is allowed only once"
            );
        }

        Ok(())
    }

    fn parse_meta_list(&mut self, meta: &MetaList) -> syn::Result<()> {
        let mut args = meta.parse_args::<Args>()?;
        args.set_span(meta.tokens.span());

        *self = args;

        Ok(())
    }

    fn to_tokens_without_black_box(&self) -> TokenStream2 {
        if let Some((span, exprs)) = self.0.as_ref() {
            quote_spanned! { *span => #(#exprs),* }
        } else {
            TokenStream2::new()
        }
    }

    /// Emit a compiler error if the number of actual and expected arguments do not match
    ///
    /// If there is a setup function present, we do not perform any checks.
    fn check_num_arguments(&self, expected: usize, has_setup: bool) {
        let actual = self.len();

        if !has_setup && actual != expected {
            if let Some(span) = self.span() {
                emit_error!(
                    span,
                    "Expected {} arguments but found {}",
                    expected,
                    actual;
                    help = "This argument is expected to have the same amount of parameters as the benchmark function";
                );
            } else {
                emit_error!(
                    self,
                    "Expected {} arguments but found {}",
                    expected,
                    actual;
                    help = "This argument is expected to have the same amount of parameters as the benchmark function";
                );
            }
        };
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let data = input
            .parse_terminated(Parse::parse, Token![,])?
            .into_iter()
            .collect();

        // We set a default span here although it is most likely wrong. It's strongly advised to set
        // the span with `Args::set_span` to the correct value.
        Ok(Self::new(input.span(), data))
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        if let Some((span, exprs)) = self.0.as_ref() {
            let this_tokens = quote_spanned! { *span => #(std::hint::black_box(#exprs)),* };
            tokens.append_all(this_tokens);
        }
    }
}

impl Bench {
    fn render_as_code(&self, callee: &Ident) -> TokenStream2 {
        let id = &self.id;
        let args = &self.args;

        let inner = self.setup.render_as_code(args);
        let call = quote! { std::hint::black_box(__iai_callgrind_wrapper_mod::#callee(#inner)) };

        let call = self.teardown.render_as_code(call);

        let func = quote!(
            #[inline(never)]
            pub fn #id() {
                let _ = #call;
            }
        );

        let config = self.config.render_as_code(id);
        quote! {
            #config
            #func
        }
    }

    fn render_as_member(&self) -> TokenStream2 {
        let id = &self.id;
        let id_display = self.id.to_string();
        let args_display = self.setup.to_string(&self.args);
        let config = self.config.render_as_member(id);
        quote! {
            iai_callgrind::internal::InternalMacroLibBench {
                id_display: Some(#id_display),
                args_display: Some(#args_display),
                func: #id,
                config: #config
            }
        }
    }
}

impl BenchConfig {
    fn ident(id: &Ident) -> Ident {
        format_ident!("get_config_{}", id)
    }

    fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            self.0 = Some(pair.value.clone());
        } else {
            emit_error!(
                pair, "Duplicate argument: `config`";
                help = "`config` is allowed only once"
            );
        }
    }

    fn render_as_code(&self, id: &Ident) -> TokenStream2 {
        if let Some(config) = &self.0 {
            let ident = Self::ident(id);
            quote! {
                #[inline(never)]
                pub fn #ident() -> iai_callgrind::internal::InternalLibraryBenchmarkConfig {
                    #config.into()
                }
            }
        } else {
            TokenStream2::new()
        }
    }

    fn render_as_member(&self, id: &Ident) -> TokenStream2 {
        if self.0.is_some() {
            let ident = Self::ident(id);
            quote! { Some(#ident) }
        } else {
            quote! { None }
        }
    }
}

impl File {
    fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            if let Expr::Lit(literal) = &pair.value {
                self.0 = Some(parse2::<LitStr>(literal.to_token_stream()).unwrap());
            } else {
                abort!(
                    pair.value, "Invalid value for `file`";
                    help = "The `file` argument needs a literal string containing the path to an existing file at compile time";
                    note = "`file = \"benches/some_fixture\"`"
                );
            }
        } else {
            emit_error!(
                pair, "Duplicate argument: `file`";
                help = "`file` is allowed only once"
            );
        }
    }

    /// Read this [`File`] and return all its lines
    ///
    /// # Panics
    ///
    /// Panics if there is no path present
    fn read(&self, cargo_meta: Option<&CargoMetadata>) -> Vec<String> {
        let expr = self.0.as_ref().expect("A file should be present");
        let string = expr.value();
        let mut path = PathBuf::from(&string);

        if path.is_relative() {
            if let Some(cargo_meta) = cargo_meta {
                let root = PathBuf::from(&cargo_meta.workspace_root);
                path = root.join(path);
            }
        }

        let file = StdFile::open(&path)
            .unwrap_or_else(|error| abort!(expr, "Error opening '{}': {}", path.display(), error));

        let mut lines = vec![];
        for (index, line) in BufReader::new(file).lines().enumerate() {
            match line {
                Ok(line) => {
                    lines.push(line);
                }
                Err(error) => {
                    abort!(
                        self.0,
                        "Error reading line {} in file '{}': {}",
                        index + 1,
                        path.display(),
                        error
                    );
                }
            }
        }
        lines
    }
}

impl LibraryBenchmark {
    fn parse_bench_attribute(
        &mut self,
        item_fn: &ItemFn,
        attr: &Attribute,
        id: Ident,
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut args = Args::default();
        let mut config = BenchConfig::default();
        let mut setup = Setup::default();
        let mut teardown = Teardown::default();

        if let Ok(pairs) =
            meta.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        {
            for pair in pairs {
                if pair.path.is_ident("args") {
                    args.parse_pair(&pair)?;
                } else if pair.path.is_ident("config") {
                    config.parse_pair(&pair);
                } else if pair.path.is_ident("setup") {
                    setup.parse_pair(&pair);
                } else if pair.path.is_ident("teardown") {
                    teardown.parse_pair(&pair);
                } else {
                    abort!(
                        pair, "Invalid argument: {}", pair.path.require_ident()?;
                        help = "Valid arguments are: `args`, `config`, `setup`, teardown`"
                    );
                }
            }
        } else {
            args.parse_meta_list(meta)?;
        }

        setup.update(&self.setup);
        teardown.update(&self.teardown);

        args.check_num_arguments(expected_num_args, setup.is_some());

        self.benches.push(Bench {
            id,
            args,
            config,
            setup,
            teardown,
        });

        Ok(())
    }

    fn parse_benches_attribute(
        &mut self,
        item_fn: &ItemFn,
        attr: &Attribute,
        id: &Ident,
        cargo_meta: Option<&CargoMetadata>,
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut config = BenchConfig::default();
        let mut setup = Setup::default();
        let mut teardown = Teardown::default();
        let mut args = MultipleArgs::default();
        let mut file = File::default();

        if let Ok(pairs) =
            meta.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        {
            for pair in pairs {
                if pair.path.is_ident("args") {
                    args.parse_pair(&pair)?;
                } else if pair.path.is_ident("config") {
                    config.parse_pair(&pair);
                } else if pair.path.is_ident("setup") {
                    setup.parse_pair(&pair);
                } else if pair.path.is_ident("teardown") {
                    teardown.parse_pair(&pair);
                } else if pair.path.is_ident("file") {
                    file.parse_pair(&pair);
                } else {
                    abort!(
                        pair, "Invalid argument: {}", pair.path.require_ident()?;
                        help = "Valid arguments are: `args`, `file`, `config`, `setup`, `teardown`"
                    );
                }
            }
        } else {
            args = MultipleArgs::from_meta_list(meta)?;
        };

        setup.update(&self.setup);
        teardown.update(&self.teardown);

        match (&file.0, args.0) {
            (Some(literal), Some(_)) => {
                abort!(
                    literal.span(),
                    "Only one parameter of `file` or `args` can be present"
                );
            }
            (None, Some(mut args)) => {
                // Make sure there is at least one `Args` present.
                //
                // `#[benches::id(args = [])]` has to result in a single Bench with an empty Args.
                if args.is_empty() {
                    args.push(Args::default());
                }
                for (i, args) in args.into_iter().enumerate() {
                    args.check_num_arguments(expected_num_args, setup.is_some());

                    let id = format_ident!("{id}_{i}");
                    self.benches.push(Bench {
                        id,
                        args,
                        config: config.clone(),
                        setup: setup.clone(),
                        teardown: teardown.clone(),
                    });
                }
            }
            (Some(literal), None) => {
                let strings = file.read(cargo_meta);
                if strings.is_empty() {
                    abort!(literal, "The provided file '{}' was empty", literal.value());
                }
                for (i, string) in strings.into_iter().enumerate() {
                    let id = format_ident!("{id}_{i}");
                    let expr = if string.is_empty() {
                        parse_quote_spanned! { literal.span() => String::new() }
                    } else {
                        parse_quote_spanned! { literal.span() => String::from(#string) }
                    };
                    let args = Args::new(literal.span(), vec![expr]);
                    self.benches.push(Bench {
                        id,
                        args,
                        config: config.clone(),
                        setup: setup.clone(),
                        teardown: teardown.clone(),
                    });
                }
            }
            // Cover the case when no arguments were present for example `#[benches::id()]`,
            (None, None) => self.benches.push(Bench {
                id: id.clone(),
                args: Args::default(),
                config: config.clone(),
                setup: setup.clone(),
                teardown: teardown.clone(),
            }),
        }

        Ok(())
    }

    fn extract_benches(
        &mut self,
        item_fn: &ItemFn,
        cargo_meta: Option<&CargoMetadata>,
    ) -> syn::Result<()> {
        let bench: syn::PathSegment = parse_quote!(bench);
        let benches: syn::PathSegment = parse_quote!(benches);

        for attr in &item_fn.attrs {
            let mut path_segments = attr.path().segments.iter();
            match path_segments.next() {
                Some(segment) if segment == &bench => {
                    if attr.path().segments.len() > 2 {
                        abort!(
                            attr, "Only one id is allowed";
                            help = "bench followed by :: and a single unique id";
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
                            help = "benches followed by :: and a single unique id";
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
                    self.parse_benches_attribute(item_fn, attr, &id, cargo_meta)?;
                }
                Some(segment) => {
                    abort!(
                        attr, "Invalid attribute: '{}'", segment.ident;
                        help = "Only the `bench` and the `benches` attribute are allowed";
                        note = r#"#[bench::my_id("with", "args")]
                    or #[benches::my_id(args = [("with", "args"), ...])]"#
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

    /// Render the `#[library_benchmark]` attribute when no outer attribute was present
    ///
    /// ```ignore
    /// #[library_benchmark]
    /// fn my_benchmark_function() -> u64 {
    ///     my_lib::bench_me(42)
    /// }
    /// ```
    fn render_standalone(self, item_fn: &ItemFn) -> TokenStream2 {
        let ident = &item_fn.sig.ident;
        let visibility: syn::Visibility = parse_quote! { pub(super) };
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: visibility,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let config = self.config.render_as_code();

        let inner = self.setup.render_as_code(&Args::default());
        let call = quote! { std::hint::black_box(__iai_callgrind_wrapper_mod::#ident(#inner)) };

        let call = self.teardown.render_as_code(call);
        quote! {
            mod #ident {
                use super::*;

                mod __iai_callgrind_wrapper_mod {
                    use super::*;

                    #[inline(never)]
                    #new_item_fn
                }

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
                    let _ = #call;
                }
            }
        }
    }

    /// Render the `#[library_benchmark]` when other outer attributes like `#[bench]` were present
    ///
    /// We use the function name of the annotated function as module name. This new module
    /// encloses the new functions generated from the `#[bench]` and `#[benches]` attribute as well
    /// as the original and unmodified benchmark function.
    ///
    /// The original benchmark function receives additional attributes `#[inline(never)]` to prevent
    /// the compiler from inlining this function. We also wrap the benchmark function into an extra
    /// module. The main problem is that the compiler replaces functions with identical body. For
    /// example the functions
    ///
    /// ```ignore
    /// #[library_benchmark]
    /// #[bench::my_id(42)]
    /// fn my_bench(arg: u64) -> u64 {
    ///     my_lib::bench_me()
    /// }
    ///
    /// #[library_benchmark]
    /// #[bench::my_id(84)]
    /// fn my_bench_with_longer_function_name(arg: u64) -> u64 {
    ///     my_lib::bench_me()
    /// }
    /// ```
    ///
    /// would be treated by the compiler as a single function (it takes the one with the shorter
    /// function name, here `my_bench`). If we don't wrap them into a module with a constant export
    /// name, we wouldn't be able to match for
    /// `my_bench_with_longer_function_name::my_bench_with_longer_function_name` since this function
    /// was replaced by the compiler with `my_bench::my_bench`.
    ///
    /// Next, we store all necessary information in a `BENCHES` slice of
    /// `iai_callgrind::internal::InternalMacroLibBench` structs. This slice can be easily accessed
    /// by the macros of the `iai-callgrind` package in which we finally can call all the benchmark
    /// functions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[library_benchmark]
    /// #[bench::my_id(42)]
    /// fn my_benchmark_function(arg: u64) -> u64 {
    ///     my_lib::bench_me(arg)
    /// }
    /// ```
    fn render_benches(self, item_fn: &ItemFn) -> TokenStream2 {
        let visibility: syn::Visibility = parse_quote! { pub(super) };
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: visibility,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let mod_name = &item_fn.sig.ident;
        let callee = &item_fn.sig.ident;
        let mut funcs = TokenStream2::new();
        let mut lib_benches = vec![];
        for bench in self.benches {
            funcs.append_all(bench.render_as_code(callee));
            lib_benches.push(bench.render_as_member());
        }

        let config = self.config.render_as_code();
        quote! {
            mod #mod_name {
                use super::*;

                mod __iai_callgrind_wrapper_mod {
                    use super::*;

                    #[inline(never)]
                    #new_item_fn
                }

                pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench] = &[
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
            let mut config = LibraryBenchmarkConfig::default();
            let mut setup = Setup::default();
            let mut teardown = Teardown::default();

            let pairs = input.parse_terminated(MetaNameValue::parse, Token![,])?;
            for pair in pairs {
                if pair.path.is_ident("config") {
                    config.parse_pair(&pair);
                } else if pair.path.is_ident("setup") {
                    setup.parse_pair(&pair);
                } else if pair.path.is_ident("teardown") {
                    teardown.parse_pair(&pair);
                } else {
                    abort!(
                        pair, "Invalid argument: {}", pair.path.require_ident()?;
                        help = "Valid arguments are: `config`, `setup`, `teardown`"
                    );
                }
            }

            let library_benchmark = LibraryBenchmark {
                config,
                setup,
                teardown,
                benches: vec![],
            };
            Ok(library_benchmark)
        }
    }
}

impl LibraryBenchmarkConfig {
    fn parse_pair(&mut self, pair: &MetaNameValue) {
        let mut config = BenchConfig::default();
        config.parse_pair(pair);
        self.0 = config.0;
    }

    fn render_as_code(&self) -> TokenStream2 {
        if let Some(config) = &self.0 {
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
                pub fn get_config()
                -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig> {
                    None
                }
            )
        }
    }
}

impl MultipleArgs {
    fn parse_pair(&mut self, pair: &MetaNameValue) -> syn::Result<()> {
        if self.0.is_none() {
            *self = MultipleArgs::from_expr(&pair.value)?;
        } else {
            abort!(
                pair, "Duplicate argument: `args`";
                help = "`args` is allowed only once"
            );
        }

        Ok(())
    }

    fn from_expr(expr: &Expr) -> syn::Result<Self> {
        let expr_array = parse2::<ExprArray>(expr.to_token_stream())?;
        let mut values: Vec<Args> = vec![];
        for elem in expr_array.elems {
            let span = elem.span();
            let args = match elem {
                Expr::Tuple(items) => {
                    let mut args = parse2::<Args>(items.elems.to_token_stream())?;
                    args.set_span(span);
                    args
                }
                Expr::Paren(item) => Args::new(span, vec![*item.expr]),
                _ => Args::new(span, vec![elem]),
            };

            values.push(args);
        }
        Ok(Self(Some(values)))
    }

    fn from_meta_list(meta: &MetaList) -> syn::Result<Self> {
        let list = &meta.tokens;
        let expr = parse2::<Expr>(quote_spanned! { list.span() => [#list] })?;
        Self::from_expr(&expr)
    }
}

impl Setup {
    fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            let expr = &pair.value;
            if let Expr::Path(path) = expr {
                self.0 = Some(path.clone());
            } else {
                abort!(
                    expr, "Invalid value for `setup`";
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
    }

    fn to_string(&self, args: &Args) -> String {
        let tokens = args.to_tokens_without_black_box();
        if let Some(setup) = self.0.as_ref() {
            quote! { #setup(#tokens) }.to_string()
        } else {
            tokens.to_string()
        }
    }

    fn render_as_code(&self, args: &Args) -> TokenStream2 {
        if let Some(setup) = &self.0 {
            quote_spanned! { setup.span() => std::hint::black_box(#setup(#args)) }
        } else {
            quote! { #args }
        }
    }

    fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// If this Setup is none and the other setup has a value update this `Setup` with that value
    fn update(&mut self, other: &Self) {
        if let (None, Some(other)) = (&self.0, &other.0) {
            self.0 = Some(other.clone());
        }
    }
}

impl Teardown {
    fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            let expr = &pair.value;
            if let Expr::Path(path) = expr {
                self.0 = Some(path.clone());
            } else {
                abort!(
                    expr, "Invalid value for `teardown`";
                    help = "The `teardown` argument needs a path to an existing function
                in a reachable scope";
                    note = "`teardown = my_teardown` or `teardown = my::teardown::function`"
                );
            }
        } else {
            abort!(
                pair, "Duplicate argument: `teardown`";
                help = "`teardown` is allowed only once"
            );
        }
    }

    fn render_as_code(&self, tokens: TokenStream2) -> TokenStream2 {
        if let Some(teardown) = &self.0 {
            quote_spanned! { teardown.span() => std::hint::black_box(#teardown(#tokens)) }
        } else {
            tokens
        }
    }

    /// If this Teardown is none and the other Teardown has a value update this Teardown with that
    /// value
    fn update(&mut self, other: &Self) {
        if let (None, Some(other)) = (&self.0, &other.0) {
            self.0 = Some(other.clone());
        }
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
    match render_library_benchmark(args.into(), input.into()) {
        Ok(stream) => stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn render_library_benchmark(args: TokenStream2, input: TokenStream2) -> syn::Result<TokenStream2> {
    let mut library_benchmark = parse2::<LibraryBenchmark>(args)?;
    let item_fn = parse2::<ItemFn>(input)?;

    let cargo_meta: Option<CargoMetadata> =
        std::process::Command::new(option_env!("CARGO").unwrap_or("cargo"))
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output()
            .ok()
            .and_then(|output| serde_json::de::from_slice(&output.stdout).ok());

    library_benchmark.extract_benches(&item_fn, cargo_meta.as_ref())?;
    if library_benchmark.benches.is_empty() {
        Ok(library_benchmark.render_standalone(&item_fn))
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

        let visibility = parse_quote! { pub(super) };
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: visibility,
            sig: func.sig.clone(),
            block: func.block.clone(),
        };

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
                pub fn get_config()
                -> Option<iai_callgrind::internal::InternalLibraryBenchmarkConfig> {
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
                    let _ = std::hint::black_box(__iai_callgrind_wrapper_mod::#callee(
                        #(std::hint::black_box(#args)),*
                    ));
                }
            ));
        }
        parse_quote!(
            mod #callee {
                use super::*;

                mod __iai_callgrind_wrapper_mod {
                    use super::*;

                    #[inline(never)]
                    #new_item_fn
                }

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
                        args_display: Some("1 , 2"),
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
