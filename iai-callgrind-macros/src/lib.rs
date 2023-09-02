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

/// TODO: DOCUMENTATION
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
