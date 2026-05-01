use super::*;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemStruct, Type};

pub fn generate(
    model: &ItemStruct, timestamp: &Option<Ident>, group_by: &Vec<Ident>, float_round: Option<f32>, table_name: &str, using_chrono: bool,
) -> proc_macro2::TokenStream {
    let name = model.ident.clone();

    let mut values = Vec::new();
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty_original = field.ty.clone();
        let mut ty = field.ty.clone();
        let round_float_field = float_round.is_some() && quote! { #ty }.to_string().starts_with("f");
        if round_float_field {
            ty = Type::Verbatim(quote! { i64 });
        }
        if quote! { #ty_original }.to_string() == "bool" {
            ty = Type::Verbatim(quote! { u16 });
        }
        if group_by.iter().any(|i| *i == ident) {
            values.push(quote! { #ident: &rows[0].#ident, });
        } else if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            values.push(quote! {
                start_at, end_at,
                #timestamp: &::pco::standalone::simple_compress(&#timestamp, &::pco::ChunkConfig::default()).unwrap(),
            });
        } else if is_number(&ty) || is_nested_number(&ty) {
            let val = if is_number(&ty) {
                quote! { r.#ident }
            } else {
                quote! { v } // Closure argument inside of nested `map`
            };
            let expr = if round_float_field {
                quote! { (#val * #float_round as #ty_original).round() as i64 }
            } else if quote! { #ty_original }.to_string() == "bool" {
                quote! { #val as u16 }
            } else {
                quote! { #val }
            };
            if is_number(&ty) {
                values.push(quote! {
                    #ident: &::pco::standalone::simple_compress(
                        &rows.iter().map(|r| #expr).collect::<Vec<_>>(), &::pco::ChunkConfig::default()
                    )?,
                });
            } else {
                values.push(quote! {
                    #ident: &pco_compress_nested(
                        rows.iter().map(|r| r.#ident.iter().map(|v| *#expr).collect::<Vec<_>>()).collect::<Vec<_>>()
                    )?,
                });
            }
        } else {
            values.push(quote! {
                #ident: &serde_compress(rows.iter().map(|r| r.#ident.clone()).collect::<Vec<_>>())?,
            });
        }

    }
    let values = tokens(values);

    let map_inner = if using_chrono {
        quote! { t.timestamp_micros() as u64 }
    } else {
        quote! { t.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64 }
    };


    let timestamp_collect = if timestamp.is_some() {
        quote! {
            let #timestamp: Vec<_> = rows.iter().map(|s| s.#timestamp).collect();
            let start_at = *#timestamp.iter().min().unwrap();
            let end_at = *#timestamp.iter().max().unwrap();
            let #timestamp: Vec<u64> = #timestamp.into_iter().map(|t| #map_inner).collect();
        }
    } else {
        quote! {}
    };

    quote! {
        pub fn new(rows: &Vec<#name>) -> anyhow::Result<Self> {
            #timestamp_collect

            Ok(Self {
                #values
                filter: None, // FIXME: I'm like 99% sure this is wrong.
            })
        }
    }
}
