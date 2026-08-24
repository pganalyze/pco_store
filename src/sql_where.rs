use super::*;
use quote::ToTokens;
use syn::Field;
use syn::punctuated::Punctuated;
use syn::token::Comma;

pub fn generate(fields: Punctuated<Field, Comma>, index_fields: &[Ident], timestamp: Option<&Ident>) -> Vec<proc_macro2::TokenStream> {
    let mut clauses: Vec<_> = index_fields
        .iter()
        .filter_map(|ident| fields.iter().find(|f| f.ident.as_ref() == Some(ident)).map(|field_def| index_filter(ident, &field_def.ty)))
        .collect();
    if let Some(ts_ident) = timestamp {
        clauses.push(quote! {
            if let Some(f) = filter.#ts_ident.as_ref() {
                match f {
                    pco_pack::DateTimeFilter::Equal(v) => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(v);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(v);
                    }
                    pco_pack::DateTimeFilter::Inclusion(vals) => {
                        if !vals.is_empty() {
                            where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                            params.push(vals.first().unwrap());
                            where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                            params.push(vals.last().unwrap());
                        }
                    }
                    pco_pack::DateTimeFilter::Range { start, end } => {
                        where_clauses.push(format!("end_at >= ${}", params.len() + 1));
                        params.push(start);
                        where_clauses.push(format!("start_at <= ${}", params.len() + 1));
                        params.push(end);
                    }
                }
            }
        });
    }
    clauses
}

fn index_filter(field: &Ident, rust_ty: &syn::Type) -> proc_macro2::TokenStream {
    let name = format!("{}", field);
    let ty_str = rust_ty.to_token_stream().to_string();
    let match_arms = if ty_str.contains("i64") {
        build_i64_filter(&name)
    } else if ty_str.contains("i32") {
        build_i32_filter(&name)
    } else if ty_str.contains("f64") {
        build_f64_filter(&name)
    } else if ty_str == "String" || ty_str.replace(" ", "").contains("SmolStr") {
        build_string_filter(&name)
    } else if ty_str.contains("Uuid") {
        build_uuid_filter(&name)
    } else if ty_str.contains("bool") {
        build_bool_filter(&name)
    } else {
        panic!("unsupported index data type: {ty_str}");
    };
    quote! {
        if let Some(f) = filter.#field.as_ref() {
            match f {
                #match_arms
            }
        }
    }
}

fn build_i64_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::I64Filter::Equal(v) => {
            where_clauses.push(format!("{} = ${}", #name, params.len() + 1));
            params.push(v);
        }
        pco_pack::I64Filter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${})", #name, params.len() + 1));
                params.push(vals);
            }
        }
        pco_pack::I64Filter::Range { start, end } => {
            where_clauses.push(format!("{} >= ${} AND {} <= ${}", #name, params.len() + 1, #name, params.len() + 2));
            params.push(start);
            params.push(end);
        }
    }
}

fn build_i32_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::I64Filter::Equal(val) => {
            where_clauses.push(format!("{} = ${}::bigint", #name, params.len() + 1));
            params.push(val);
        }
        pco_pack::I64Filter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${}::bigint[])", #name, params.len() + 1));
                params.push(vals);
            }
        }
        pco_pack::I64Filter::Range { start, end } => {
            where_clauses.push(format!("{} >= ${}::bigint AND {} <= ${}::bigint", #name, params.len() + 1, #name, params.len() + 2));
            params.push(start);
            params.push(end);
        }
    }
}

fn build_f64_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::F64Filter::Equal(v) => {
            where_clauses.push(format!("{} = ${}", #name, params.len() + 1));
            params.push(v);
        }
        pco_pack::F64Filter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${})", #name, params.len() + 1));
                params.push(vals);
            }
        }
        pco_pack::F64Filter::Range { start, end } => {
            where_clauses.push(format!("{} >= ${} AND {} <= ${}", #name, params.len() + 1, #name, params.len() + 2));
            params.push(start);
            params.push(end);
        }
    }
}

fn build_string_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::StringFilter::Equal(v) => {
            where_clauses.push(format!("{} = ${}", #name, params.len() + 1));
            params.push(v);
        }
        pco_pack::StringFilter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${})", #name, params.len() + 1));
                params.push(vals);
            }
        }
    }
}

fn build_uuid_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::UuidFilter::Equal(v) => {
            where_clauses.push(format!("{} = ${}", #name, params.len() + 1));
            params.push(v);
        }
        pco_pack::UuidFilter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${})", #name, params.len() + 1));
                params.push(vals);
            }
        }
    }
}

fn build_bool_filter(name: &str) -> proc_macro2::TokenStream {
    quote! {
        pco_pack::BoolFilter::Equal(v) => {
            where_clauses.push(format!("{} = ${}", #name, params.len() + 1));
            params.push(v);
        }
        pco_pack::BoolFilter::Inclusion(vals) => {
            if !vals.is_empty() {
                where_clauses.push(format!("{} = ANY(${})", #name, params.len() + 1));
                params.push(vals);
            }
        }
    }
}
