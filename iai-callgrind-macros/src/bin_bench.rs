use std::fmt::Display;
use std::ops::Deref;

use derive_more::{Deref, DerefMut};
use proc_macro2::TokenStream;
use proc_macro_error2::abort;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse2, parse_quote, Attribute, Expr, Ident, ItemFn, MetaNameValue, Token};

use crate::common::{self, format_ident, truncate_str_utf8, BenchesArgs, File};
use crate::{defaults, CargoMetadata};

/// This struct reflects the `args` parameter of the `#[bench]` attribute
#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct Args(common::Args);

/// This is the counterpart for the `#[bench]` attribute
///
/// The `#[benches]` attribute is also parsed into this structure.
#[derive(Debug)]
struct Bench {
    id: Ident,
    args: Args,
    config: BenchConfig,
    setup: Setup,
    teardown: Teardown,
}

#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct BenchConfig(common::BenchConfig);

/// This is the counterpart to the `#[binary_benchmark]` attribute.
#[derive(Debug, Default)]
struct BinaryBenchmark {
    config: BinaryBenchmarkConfig,
    setup: Setup,
    teardown: Teardown,
    benches: Vec<Bench>,
}

/// The `config` parameter of the `#[binary_benchmark]` attribute
///
/// The `BenchConfig` and `BinaryBenchmarkConfig` are rendered differently, hence the different
/// structures
///
/// Note: This struct is completely independent of the `iai_callgrind::BinaryBenchmarkConfig`
/// struct with the same name.
#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct BinaryBenchmarkConfig(common::BenchConfig);

#[derive(Debug, Default, Clone)]
struct Setup(Option<Expr>);

#[derive(Debug, Default, Clone)]
struct Teardown(Option<Expr>);

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.deref().to_tokens(tokens);
    }
}

impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tokens = self.to_tokens_without_black_box().to_string();
        write!(f, "{tokens}")
    }
}

impl Bench {
    fn parse_bench_attribute(
        item_fn: &ItemFn,
        attr: &Attribute,
        id: Ident,
        other_setup: &Setup,
        other_teardown: &Teardown,
    ) -> syn::Result<Self> {
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

        setup.update(other_setup);
        teardown.update(other_teardown);

        args.check_num_arguments(expected_num_args, setup.is_some());

        Ok(Bench {
            id,
            args,
            config,
            setup,
            teardown,
        })
    }

    fn parse_benches_attribute(
        item_fn: &ItemFn,
        attr: &Attribute,
        id: &Ident,
        other_setup: &Setup,
        other_teardown: &Teardown,
        cargo_meta: Option<&CargoMetadata>,
    ) -> syn::Result<Vec<Self>> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut config = BenchConfig::default();
        let mut setup = Setup::default();
        let mut teardown = Teardown::default();
        let mut args = BenchesArgs::default();
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
            args = BenchesArgs::from_meta_list(meta)?;
        };

        setup.update(other_setup);
        teardown.update(other_teardown);

        let benches = common::Bench::from_benches_attribute(
            id,
            args,
            &file,
            cargo_meta,
            setup.is_some(),
            expected_num_args,
        )
        .into_iter()
        .map(|b| Bench {
            id: b.id,
            args: Args(b.args),
            config: config.clone(),
            setup: setup.clone(),
            teardown: teardown.clone(),
        })
        .collect();

        Ok(benches)
    }

    fn render_as_code(&self, callee: &Ident) -> TokenStream {
        let id = &self.id;
        let args = &self.args;

        let func = quote!(
            #[inline(never)]
            pub fn #id() -> iai_callgrind::Command {
                #callee(#args)
            }
        );

        let config = self.config.render_as_code(Some(id));
        let setup = self.setup.render_as_code(Some(id), &self.args);
        let teardown = self.teardown.render_as_code(Some(id), &self.args);

        quote! {
            #config
            #setup
            #teardown
            #func
        }
    }

    fn render_as_member(&self) -> TokenStream {
        let id = &self.id;
        let id_display = self.id.to_string();
        let args_string = self.args.to_string();
        let args_display = truncate_str_utf8(&args_string, defaults::MAX_BYTES_ARGS);
        let config = self.config.render_as_member(Some(id));
        let setup = self.setup.render_as_member(Some(id));
        let teardown = self.teardown.render_as_member(Some(id));
        quote! {
            iai_callgrind::internal::InternalMacroBinBench {
                id_display: Some(#id_display),
                args_display: Some(#args_display),
                func: #id,
                config: #config,
                setup: #setup,
                teardown: #teardown,
            }
        }
    }
}

impl BenchConfig {
    pub fn ident(id: Option<&Ident>) -> Ident {
        format_ident("__get_config", id)
    }

    fn render_as_code(&self, id: Option<&Ident>) -> TokenStream {
        if let Some(config) = &self.deref().0 {
            let ident = Self::ident(id);
            quote! {
                #[inline(never)]
                pub fn #ident() -> iai_callgrind::internal::InternalBinaryBenchmarkConfig {
                    #config.into()
                }
            }
        } else {
            TokenStream::new()
        }
    }

    fn render_as_member(&self, id: Option<&Ident>) -> TokenStream {
        if self.deref().is_some() {
            let ident = Self::ident(id);
            quote! { Some(#ident) }
        } else {
            quote! { None }
        }
    }
}

impl BinaryBenchmark {
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
                    self.benches.push(Bench::parse_bench_attribute(
                        item_fn,
                        attr,
                        id,
                        &self.setup,
                        &self.teardown,
                    )?);
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
                    self.benches.extend(Bench::parse_benches_attribute(
                        item_fn,
                        attr,
                        &id,
                        &self.setup,
                        &self.teardown,
                        cargo_meta,
                    )?);
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

    /// Render the `#[binary_benchmark]` attribute when no outer attribute was present
    ///
    /// ```ignore
    /// #[binary_benchmark]
    /// fn my_benchmark_function() -> u64 {
    ///     my_lib::bench_me(42)
    /// }
    /// ```
    fn render_standalone(self, item_fn: &ItemFn) -> TokenStream {
        let ident = &item_fn.sig.ident;
        let visibility: syn::Visibility = parse_quote! { pub };
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: visibility,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let config = self.config.render_as_code();
        let setup = self.setup.render_as_code(None, &Args::default());
        let setup_member = self.setup.render_as_member(None);
        let teardown = self.teardown.render_as_code(None, &Args::default());
        let teardown_member = self.teardown.render_as_member(None);

        quote! {
            mod #ident {
                use super::*;

                #[inline(never)]
                #new_item_fn

                pub const __BENCHES: &[iai_callgrind::internal::InternalMacroBinBench]= &[
                    iai_callgrind::internal::InternalMacroBinBench {
                        id_display: None,
                        args_display: None,
                        func: #ident,
                        setup: #setup_member,
                        teardown: #teardown_member,
                        config: None
                    },
                ];

                #config
                #setup
                #teardown
            }
        }
    }

    fn render_benches(self, item_fn: &ItemFn) -> TokenStream {
        let new_item_fn = ItemFn {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            sig: item_fn.sig.clone(),
            block: item_fn.block.clone(),
        };

        let mod_name = &item_fn.sig.ident;
        let callee = &item_fn.sig.ident;
        let mut funcs = TokenStream::new();
        let mut bin_benches = vec![];
        for bench in self.benches {
            funcs.append_all(bench.render_as_code(callee));
            bin_benches.push(bench.render_as_member());
        }

        let config = self.config.render_as_code();
        quote! {
            mod #mod_name {
                use super::*;

                #[inline(never)]
                #new_item_fn

                pub const __BENCHES: &[iai_callgrind::internal::InternalMacroBinBench] = &[
                    #(#bin_benches,)*
                ];

                #config

                #funcs
            }
        }
    }
}

impl Parse for BinaryBenchmark {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self::default())
        } else {
            let mut config = BinaryBenchmarkConfig::default();
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

            let binary_benchmark = Self {
                config,
                setup,
                teardown,
                benches: vec![],
            };
            Ok(binary_benchmark)
        }
    }
}

impl BinaryBenchmarkConfig {
    fn render_as_code(&self) -> TokenStream {
        if let Some(config) = &self.deref().0 {
            quote!(
                #[inline(never)]
                pub fn __get_config()
                    -> Option<iai_callgrind::internal::InternalBinaryBenchmarkConfig>
                {
                    Some(#config.into())
                }
            )
        } else {
            quote!(
                #[inline(never)]
                pub fn __get_config(
                ) -> Option<iai_callgrind::internal::InternalBinaryBenchmarkConfig>
                {
                    None
                }
            )
        }
    }
}

impl Setup {
    pub fn ident(id: Option<&Ident>) -> Ident {
        format_ident("__setup", id)
    }

    pub fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            self.0 = Some(pair.value.clone());
        } else {
            abort!(
                pair, "Duplicate argument: `setup`";
                help = "`setup` is allowed only once"
            );
        }
    }

    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// If this Setup is none and the other setup has a value update this `Setup` with that value
    pub fn update(&mut self, other: &Self) {
        if let (None, Some(other)) = (&self.0, &other.0) {
            self.0 = Some(other.clone());
        }
    }

    fn render_as_code(&self, id: Option<&Ident>, args: &Args) -> TokenStream {
        if let Some(setup) = &self.0 {
            let ident = Self::ident(id);
            let call = if let Expr::Path(path) = &setup {
                quote!(#path(#args))
            } else {
                quote!(#setup)
            };
            quote! {
                #[inline(never)]
                pub fn #ident() {
                    #call;
                }
            }
        } else {
            TokenStream::new()
        }
    }

    fn render_as_member(&self, id: Option<&Ident>) -> TokenStream {
        if self.0.is_some() {
            let ident = Self::ident(id);
            quote! { Some(#ident) }
        } else {
            quote! { None }
        }
    }
}

impl Teardown {
    pub fn ident(id: Option<&Ident>) -> Ident {
        format_ident("__teardown", id)
    }

    pub fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            self.0 = Some(pair.value.clone());
        } else {
            abort!(
                pair, "Duplicate argument: `teardown`";
                help = "`teardown` is allowed only once"
            );
        }
    }

    /// If this Setup is none and the other setup has a value update this `Setup` with that value
    pub fn update(&mut self, other: &Self) {
        if let (None, Some(other)) = (&self.0, &other.0) {
            self.0 = Some(other.clone());
        }
    }

    fn render_as_code(&self, id: Option<&Ident>, args: &Args) -> TokenStream {
        if let Some(teardown) = &self.0 {
            let ident = Self::ident(id);
            let call = if let Expr::Path(path) = &teardown {
                quote!(#path(#args))
            } else {
                quote!(#teardown)
            };
            quote! {
                #[inline(never)]
                pub fn #ident() {
                    #call;
                }
            }
        } else {
            TokenStream::new()
        }
    }

    fn render_as_member(&self, id: Option<&Ident>) -> TokenStream {
        if self.0.is_some() {
            let ident = Self::ident(id);
            quote! { Some(#ident) }
        } else {
            quote! { None }
        }
    }
}

pub fn render(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let mut binary_benchmark = parse2::<BinaryBenchmark>(args)?;
    let item_fn = parse2::<ItemFn>(input)?;
    let cargo_meta = CargoMetadata::try_new();

    binary_benchmark.extract_benches(&item_fn, cargo_meta.as_ref())?;
    if binary_benchmark.benches.is_empty() {
        Ok(binary_benchmark.render_standalone(&item_fn))
    } else {
        Ok(binary_benchmark.render_benches(&item_fn))
    }
}
