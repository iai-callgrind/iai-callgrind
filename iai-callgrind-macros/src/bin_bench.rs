use derive_more::{Deref, DerefMut};
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::format_ident;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parse2, parse_quote, Attribute, Ident, ItemFn, MetaNameValue, Token};

use crate::common::{self, MultipleArgs};

// TODO: CHECK FOR OCCURRENCES OF library benchmark strings in docs and else

/// This struct reflects the `args` parameter of the `#[bench]` attribute
#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct Args(common::Args);

/// This is the counterpart for the `#[bench]` attribute
///
/// The #[benches] attribute is also parsed into this structure.
#[derive(Debug)]
struct Bench {
    id: Ident,
    args: Args,
    config: common::BenchConfig,
    setup: Setup,
    teardown: Teardown,
}

/// This is the counterpart to the `#[binary_benchmark]` attribute.
#[derive(Debug, Default)]
struct BinaryBenchmark {
    config: BinaryBenchmarkConfig,
    setup: Setup,
    teardown: Teardown,
    benches: Vec<Bench>,
}

/// The `config` parameter of the `#[library_benchmark]` attribute
///
/// The `BenchConfig` and `BinaryBenchmarkConfig` are rendered differently, hence the different
/// structures
///
/// Note: This struct is completely independent of the `iai_callgrind::BinaryBenchmarkConfig`
/// struct with the same name.
#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct BinaryBenchmarkConfig(common::BenchConfig);

#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct Setup(common::Setup);

#[derive(Debug, Default, Clone, Deref, DerefMut)]
struct Teardown(common::Teardown);

impl BinaryBenchmark {
    fn parse_bench_attribute(
        &mut self,
        item_fn: &ItemFn,
        attr: &Attribute,
        id: Ident,
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut args = Args::default();
        let mut config = common::BenchConfig::default();
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
    ) -> syn::Result<()> {
        let expected_num_args = item_fn.sig.inputs.len();
        let meta = attr.meta.require_list()?;

        let mut config = common::BenchConfig::default();
        let mut setup = Setup::default();
        let mut teardown = Teardown::default();
        let mut args = MultipleArgs::default();

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
                        help = "Valid arguments are: `args`, `config`, `setup`, `teardown`"
                    );
                }
            }
        } else {
            args = MultipleArgs::from_meta_list(meta)?;
        };

        setup.update(&self.setup);
        teardown.update(&self.teardown);

        // Make sure there is at least one `Args` present.
        //
        // `#[benches::id()]`, `#[benches::id(args = [])]` have to result in a single Bench with an
        // empty Args.
        let args = args.0.map_or_else(
            || vec![Args::default()],
            |a| {
                if a.is_empty() {
                    vec![Args::default()]
                } else {
                    a.into_iter().map(Args).collect()
                }
            },
        );
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
                    self.parse_benches_attribute(item_fn, attr, &id)?;
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

    // /// Render the `#[library_benchmark]` attribute when no outer attribute was present
    // ///
    // /// ```ignore
    // /// #[library_benchmark]
    // /// fn my_benchmark_function() -> u64 {
    // ///     my_lib::bench_me(42)
    // /// }
    // /// ```
    // fn render_standalone(self, item_fn: &ItemFn) -> TokenStream {
    //     let ident = &item_fn.sig.ident;
    //     let visibility: syn::Visibility = parse_quote! { pub(super) };
    //     let new_item_fn = ItemFn {
    //         attrs: vec![],
    //         vis: visibility,
    //         sig: item_fn.sig.clone(),
    //         block: item_fn.block.clone(),
    //     };
    //
    //     let config = self.config.render_as_code();
    //
    //     let inner = self.setup.render_as_code(&Args::default());
    //     let call = quote! { std::hint::black_box(__iai_callgrind_wrapper_mod::#ident(#inner)) };
    //
    //     let call = self.teardown.render_as_code(call);
    //     quote! {
    //         mod #ident {
    //             use super::*;
    //
    //             mod __iai_callgrind_wrapper_mod {
    //                 use super::*;
    //
    //                 #[inline(never)]
    //                 #new_item_fn
    //             }
    //
    //             pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench]= &[
    //                 iai_callgrind::internal::InternalMacroLibBench {
    //                     id_display: None,
    //                     args_display: None,
    //                     func: wrapper,
    //                     config: None
    //                 },
    //             ];
    //
    //             #config
    //
    //             #[inline(never)]
    //             pub fn wrapper() {
    //                 let _ = #call;
    //             }
    //         }
    //     }
    // }
    //
    // /// TODO: Update documentation
    // /// Render the `#[library_benchmark]` when other outer attributes like `#[bench]` were
    // present ///
    // /// We use the function name of the annotated function as module name. This new module
    // /// encloses the new functions generated from the `#[bench]` and `#[benches]` attribute as
    // well /// as the original and unmodified benchmark function.
    // ///
    // /// The original benchmark function receives additional attributes `#[inline(never)]` to
    // prevent /// the compiler from inlining this function and `#[export_name]` to export this
    // function with a /// prefix `iai_callgrind::bench::`. The latter attribute is important
    // since we extract the /// costs in the iai-callgrind-runner using callgrind's function
    // match mechanism via a wildcard /// `iai_callgrind::bench::*`. The main problem is that
    // the compiler replaces functions with /// identical body. For example the functions
    // ///
    // /// ```ignore
    // /// #[library_benchmark]
    // /// #[bench::my_id(42)]
    // /// fn my_bench(arg: u64) -> u64 {
    // ///     my_lib::bench_me()
    // /// }
    // ///
    // /// #[library_benchmark]
    // /// #[bench::my_id(84)]
    // /// fn my_bench_with_longer_function_name(arg: u64) -> u64 {
    // ///     my_lib::bench_me()
    // /// }
    // /// ```
    // ///
    // /// would be treated by the compiler as a single function (it takes the one with the shorter
    // /// function name, here `my_bench`) and both function names would be exported under the same
    // /// name. If we don't export these functions with a common prefix, we wouldn't be able to
    // /// match for `my_bench_with_longer_function_name::my_bench_with_longer_function_name` since
    // /// this function was replaced by the compiler with `my_bench::my_bench`.
    // ///
    // /// Next, we store all necessary information in a `BENCHES` slice of
    // /// `iai_callgrind::internal::InternalMacroLibBench` structs. This slice can be easily
    // accessed /// by the macros of the `iai-callgrind` package in which we finally can call
    // all the benchmark /// functions.
    // ///
    // /// # Example
    // ///
    // /// ```ignore
    // /// #[library_benchmark]
    // /// #[bench::my_id(42)]
    // /// fn my_benchmark_function(arg: u64) -> u64 {
    // ///     my_lib::bench_me(arg)
    // /// }
    // /// ```
    // fn render_benches(self, item_fn: &ItemFn) -> TokenStream {
    //     let visibility: syn::Visibility = parse_quote! { pub(super) };
    //     let new_item_fn = ItemFn {
    //         attrs: vec![],
    //         vis: visibility,
    //         sig: item_fn.sig.clone(),
    //         block: item_fn.block.clone(),
    //     };
    //
    //     let mod_name = &item_fn.sig.ident;
    //     let callee = &item_fn.sig.ident;
    //     let mut funcs = TokenStream::new();
    //     let mut lib_benches = vec![];
    //     for bench in self.benches {
    //         funcs.append_all(bench.render_as_code(callee));
    //         lib_benches.push(bench.render_as_member());
    //     }
    //
    //     let config = self.config.render_as_code();
    //     quote! {
    //         mod #mod_name {
    //             use super::*;
    //
    //             mod __iai_callgrind_wrapper_mod {
    //                 use super::*;
    //
    //                 #[inline(never)]
    //                 #new_item_fn
    //             }
    //
    //             pub const BENCHES: &[iai_callgrind::internal::InternalMacroLibBench] = &[
    //                 #(#lib_benches,)*
    //             ];
    //
    //             #config
    //
    //             #funcs
    //         }
    //     }
    // }
}

impl Parse for BinaryBenchmark {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // if input.is_empty() {
        Ok(Self::default())
        // } else {
        //     let mut config = LibraryBenchmarkConfig::default();
        //     let mut setup = Setup::default();
        //     let mut teardown = Teardown::default();
        //
        //     let pairs = input.parse_terminated(MetaNameValue::parse, Token![,])?;
        //     for pair in pairs {
        //         if pair.path.is_ident("config") {
        //             config.parse_pair(&pair);
        //         } else if pair.path.is_ident("setup") {
        //             setup.parse_pair(&pair);
        //         } else if pair.path.is_ident("teardown") {
        //             teardown.parse_pair(&pair);
        //         } else {
        //             abort!(
        //                 pair, "Invalid argument: {}", pair.path.require_ident()?;
        //                 help = "Valid arguments are: `config`, `setup`, `teardown`"
        //             );
        //         }
        //     }
        //
        //     let library_benchmark = LibraryBenchmark {
        //         config,
        //         setup,
        //         teardown,
        //         benches: vec![],
        //     };
        //     Ok(library_benchmark)
        // }
    }
}

pub fn render(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let mut binary_benchmark = parse2::<BinaryBenchmark>(args)?;
    let item_fn = parse2::<ItemFn>(input)?;

    binary_benchmark.extract_benches(&item_fn)?;
    if binary_benchmark.benches.is_empty() {
        // Ok(binary_benchmark.render_standalone(&item_fn))
        // TODO: REMOVE TEMPORARY return
        Ok(TokenStream::new())
    } else {
        // Ok(binary_benchmark.render_benches(&item_fn))
        // TODO: REMOVE TEMPORARY return
        Ok(TokenStream::new())
    }
}
