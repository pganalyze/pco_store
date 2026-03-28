use super::*;
use quote::quote;
use syn::{ItemStruct, Type};

pub fn generate(
    model: &ItemStruct, timestamp: &Option<Ident>, group_by: &Vec<Ident>, float_round: Option<f32>, _table_name: &str, using_chrono: bool,
) -> proc_macro2::TokenStream {
    let name = model.ident.clone();

    let mut decompress_fields = Vec::new();
    let mut compressed_field_sizes = Vec::new();
    let mut decompressed_fields = Vec::new();
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty_original = field.ty.clone();
        let mut ty = field.ty.clone();
        let round_float_field = float_round.is_some() && quote! { #ty }.to_string().starts_with("f");
        if group_by.iter().any(|i| *i == ident) {
            decompressed_fields.push(quote! { #ident: self.#ident.clone(), });
        } else {
            if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
                ty = Type::Verbatim(quote! { u64 });
            }
            if round_float_field {
                ty = Type::Verbatim(quote! { i64 });
            }
            if quote! { #ty_original }.to_string() == "bool" {
                ty = Type::Verbatim(quote! { u16 });
            }
            decompress_fields.push(quote! {
                let #ident: Vec<#ty> = if self.#ident.is_empty() {
                    Vec::new()
                } else {
                    ::pco::standalone::simple_decompress(&self.#ident)?
                };
            });
            compressed_field_sizes.push(quote! { #ident.len(), });
            if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
                let value = if using_chrono {
                    quote! {
                        chrono::DateTime::from_timestamp_micros(#ident[index] as i64).unwrap()
                    }
                } else {
                    quote! {
                        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_micros(#ident[index])
                    }
                };
                decompressed_fields.push(quote! {
                    #ident: #value,
                });
            } else if round_float_field {
                decompressed_fields.push(quote! {
                    #ident: #ident.get(index).cloned().unwrap_or_default() as #ty_original / #float_round as #ty_original,
                });
            } else if quote! { #ty_original }.to_string() == "bool" {
                decompressed_fields.push(quote! {
                    #ident: #ident.get(index).cloned().unwrap_or_default() == 1,
                });
            } else {
                decompressed_fields.push(quote! {
                    #ident: #ident.get(index).cloned().unwrap_or_default(),
                });
            }
        }
    }
    let decompress_fields = tokens(decompress_fields);
    let compressed_field_sizes = tokens(compressed_field_sizes);
    let decompressed_fields = tokens(decompressed_fields);

    quote! {
        /// Decompresses a group of data points.
        pub fn decompress(self) -> anyhow::Result<Vec<#name>> {
            let mut results = Vec::new();
            #decompress_fields
            let len = [#compressed_field_sizes].into_iter().max().unwrap_or(0);
            for index in 0..len {
                let row = #name { #decompressed_fields };
                if self.filter.as_ref().map(|f| f.matches(&row)) != Some(false) {
                    results.push(row);
                }
            }
            Ok(results)
        }
    }
}
