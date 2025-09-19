use proc_macro2::TokenStream;
use proc_macro_error2::abort;
use quote::quote;
use syn::parse2;

pub fn render_into_inner(input_stream: TokenStream) -> syn::Result<TokenStream> {
    let input: syn::DeriveInput = parse2(input_stream)?;
    let src = &input.ident;
    let inner = if let syn::Data::Struct(item) = &input.data {
        if let syn::Fields::Unnamed(fields) = &item.fields {
            let unnamed: Vec<syn::Field> = fields
                .unnamed
                .pairs()
                .map(|pair| pair.into_value().clone())
                .collect();
            match unnamed.as_slice() {
                [] => abort!(input, "Tuple struct must have a single field"),
                [field] => field.clone(),
                _ => abort!(input, "Only tuple structs with a single field are allowed"),
            }
        } else {
            abort!(input, "Only tuple structs with unnamed fields are allowed")
        }
    } else {
        abort!(input, "Only structs are allowed");
    };

    let expanded = quote! {
        impl From<#src> for #inner {
            fn from(value: #src) -> Self {
                value.0
            }
        }

        impl From<&#src> for #inner {
            fn from(value: &#src) -> Self {
                value.0.clone()
            }
        }

        impl From<&mut #src> for #inner {
            fn from(value: &mut #src) -> Self {
                value.0.clone()
            }
        }
    };

    Ok(expanded)
}
