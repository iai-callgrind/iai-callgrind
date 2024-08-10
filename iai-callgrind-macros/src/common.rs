use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, emit_error};
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{parse2, Expr, ExprArray, ExprPath, Ident, MetaList, MetaNameValue, Token};

/// This struct reflects the `args` parameter of the `#[bench]` attribute
#[derive(Debug, Default, Clone)]
pub struct Args(Option<(Span, Vec<Expr>)>);

/// The `config` parameter of the `#[bench]` or `#[benches]` attribute
#[derive(Debug, Default, Clone)]
pub struct BenchConfig(pub Option<Expr>);

/// This struct stores multiple `Args` as needed by the `#[benches]` attribute
#[derive(Debug, Clone, Default)]
pub struct BenchesArgs(pub Option<Vec<Args>>);

#[derive(Debug, Default, Clone)]
pub struct Setup(pub Option<ExprPath>);

#[derive(Debug, Default, Clone)]
pub struct Teardown(pub Option<ExprPath>);

impl Args {
    pub fn new(span: Span, data: Vec<Expr>) -> Self {
        Self(Some((span, data)))
    }

    pub fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |(_, data)| data.len())
    }

    pub fn span(&self) -> Option<&Span> {
        self.0.as_ref().map(|(span, _)| span)
    }

    pub fn set_span(&mut self, span: Span) {
        if let Some(data) = self.0.as_mut() {
            data.0 = span;
        }
    }

    pub fn parse_pair(&mut self, pair: &MetaNameValue) -> syn::Result<()> {
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

    pub fn parse_meta_list(&mut self, meta: &MetaList) -> syn::Result<()> {
        let mut args = meta.parse_args::<Args>()?;
        args.set_span(meta.tokens.span());

        *self = args;

        Ok(())
    }

    pub fn to_tokens_without_black_box(&self) -> TokenStream {
        if let Some((span, exprs)) = self.0.as_ref() {
            quote_spanned! { *span => #(#exprs),* }
        } else {
            TokenStream::new()
        }
    }

    /// Emit a compiler error if the number of actual and expected arguments do not match
    ///
    /// If there is a setup function present, we do not perform any checks.
    pub fn check_num_arguments(&self, expected: usize, has_setup: bool) {
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
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some((span, exprs)) = self.0.as_ref() {
            let this_tokens = quote_spanned! { *span => #(std::hint::black_box(#exprs)),* };
            tokens.append_all(this_tokens);
        }
    }
}

impl BenchesArgs {
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub fn parse_pair(&mut self, pair: &MetaNameValue) -> syn::Result<()> {
        if self.0.is_none() {
            *self = BenchesArgs::from_expr(&pair.value)?;
        } else {
            abort!(
                pair, "Duplicate argument: `args`";
                help = "`args` is allowed only once"
            );
        }

        Ok(())
    }

    pub fn from_expr(expr: &Expr) -> syn::Result<Self> {
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

    pub fn from_meta_list(meta: &MetaList) -> syn::Result<Self> {
        let list = &meta.tokens;
        let expr = parse2::<Expr>(quote_spanned! { list.span() => [#list] })?;
        Self::from_expr(&expr)
    }

    // Make sure there is at least one `Args` present then return an iterator
    //
    // `#[benches::id()]`, `#[benches::id(args = [])]` have to result in a single Bench with
    // an empty Args.
    pub fn finalize(self) -> impl Iterator<Item = Args> {
        if let Some(args) = self.0 {
            if args.is_empty() {
                vec![Args::default()].into_iter()
            } else {
                args.into_iter()
            }
        } else {
            vec![Args::default()].into_iter()
        }
    }
}

impl BenchConfig {
    pub fn ident(id: &Ident) -> Ident {
        format_ident("__get_config", Some(id))
    }

    pub fn parse_pair(&mut self, pair: &MetaNameValue) {
        if self.0.is_none() {
            self.0 = Some(pair.value.clone());
        } else {
            emit_error!(
                pair, "Duplicate argument: `config`";
                help = "`config` is allowed only once"
            );
        }
    }

    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

impl Setup {
    pub fn parse_pair(&mut self, pair: &MetaNameValue) {
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

    pub fn to_string(&self, args: &Args) -> String {
        let tokens = args.to_tokens_without_black_box();
        if let Some(setup) = self.0.as_ref() {
            quote! { #setup(#tokens) }.to_string()
        } else {
            tokens.to_string()
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
}

impl Teardown {
    pub fn parse_pair(&mut self, pair: &MetaNameValue) {
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

    /// If this Teardown is none and the other Teardown has a value update this Teardown with that
    /// value
    pub fn update(&mut self, other: &Self) {
        if let (None, Some(other)) = (&self.0, &other.0) {
            self.0 = Some(other.clone());
        }
    }
}

pub fn pretty_expr_path(expr: &ExprPath) -> String {
    expr.to_token_stream().to_string().replace(' ', "")
}

pub fn format_ident(prefix: &str, ident: Option<&Ident>) -> Ident {
    if let Some(ident) = ident {
        format_ident!("{prefix}_{ident}")
    } else {
        format_ident!("{prefix}")
    }
}

pub fn format_indexed_ident(ident: &Ident, index: usize) -> Ident {
    format_ident!("{ident}__{index}")
}

/// Truncate a utf-8 [`std::str`] to a given `len`
pub fn truncate_str_utf8(string: &str, len: usize) -> &str {
    if let Some((pos, c)) = string
        .char_indices()
        .take_while(|(i, c)| i + c.len_utf8() <= len)
        .last()
    {
        &string[..pos + c.len_utf8()]
    } else {
        &string[..0]
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use syn::parse_quote;

    use super::*;

    #[rstest]
    #[case::simple(parse_quote!(simple), "simple")]
    #[case::with_segments(parse_quote!(simple::segment), "simple::segment")]
    #[case::with_turbo_fish(parse_quote!(simple::segment::<Vec<String>>), "simple::segment::<Vec<String>>")]
    #[case::leading_colon(parse_quote!(::segment), "::segment")]
    #[case::leading_colon_multiple(parse_quote!(::simple::segment), "::simple::segment")]
    #[case::leading_colon_with_turbo_fish(parse_quote!(::simple::segment::<Vec<String>>), "::simple::segment::<Vec<String>>")]
    fn test_expr_path_to_string(#[case] expr: ExprPath, #[case] expected: &str) {
        assert_eq!(pretty_expr_path(&expr), expected);
    }
}
