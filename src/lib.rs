use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Lit, Result, Token, bracketed, parse_macro_input};

mod deserialize_time_range;
use deserialize_time_range::*;

mod fields;
use fields::*;

mod filter;
use filter::*;

mod store;
mod decompress;
mod delete;
mod load;

#[derive(Clone)]
struct Arguments {
    timestamp: Option<Ident>,
    group_by: Vec<Ident>,
    float_round: Option<f32>,
    table_name: Option<Ident>,
}
impl Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut timestamp = None;
        let mut group_by = Vec::new();
        let mut float_round = None;
        let mut table_name = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            match ident.to_string().as_str() {
                "timestamp" => timestamp = Some(input.parse()?),
                "group_by" => {
                    let content;
                    bracketed!(content in input);
                    group_by = content.parse_terminated(Ident::parse, Token![,])?.into_iter().collect();
                }
                "float_round" => {
                    if let Lit::Int(value) = Lit::parse(input)? {
                        let value = value.base10_parse()?;
                        assert!(value > 0, "float_round must be greater than zero");
                        float_round = Some(10i32.pow(value) as f32);
                    } else {
                        panic!("unsupported float_round value");
                    }
                }
                "table_name" => table_name = Some(input.parse()?),
                _ => {
                    input.error("unexpected ident");
                }
            }
            let _: Option<Token![,]> = input.parse().ok();
        }
        Ok(Self { timestamp, group_by, float_round, table_name })
    }
}

#[proc_macro_attribute]
pub fn store(args: TokenStream, item: TokenStream) -> TokenStream {
    let a = args.clone();
    let i = item.clone();
    let args = parse_macro_input!(a as Arguments);
    let Arguments { timestamp, group_by, float_round, table_name } = args.clone();
    let model = parse_macro_input!(i as ItemStruct);
    let item = proc_macro2::TokenStream::from(item);

    let name = model.ident.clone();
    let packed_name = Ident::new(&format!("Compressed{}s", model.ident), Span::call_site());

    let table_name = if let Some(table_name) = table_name {
        table_name.to_string()
    } else {
        let mut table_name = String::new();
        for c in model.ident.to_string().chars() {
            if c.is_uppercase() && table_name.len() > 0 {
                table_name += "_";
            }
            table_name += &c.to_lowercase().to_string();
        }
        table_name += "s";
        table_name
    };

    // load and delete
    let mut packed_fields = vec![quote! { filter: Option<Filter>, }];
    let mut load_checks = Vec::new();
    let mut load_where = Vec::new();
    let mut load_params = Vec::new();
    let mut bind = 1;
    let mut timestamp_ty = None;
    let mut using_chrono = false;
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        let name = format!("{ident}");
        if group_by.iter().any(|i| *i == ident) {
            packed_fields.push(quote! { #ident: #ty, });
            load_checks.push(quote! {
                if filter.#ident.is_empty() {
                    return Err(anyhow::Error::msg(#name.to_string() + " is required"));
                }
            });
            load_where.push(format!("{ident} = ANY(${bind})"));
            bind += 1;
            load_params.push(quote! { &filter.#ident, });
        } else if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            using_chrono = !ty.to_token_stream().to_string().contains("SystemTime");
            timestamp_ty = Some(ty.clone());
            packed_fields.push(quote! { #ident: Vec<u8>, });
            load_checks.push(quote! {
                if filter.#ident.is_none() {
                    return Err(anyhow::Error::msg(#name.to_string() + " is required"));
                }
                filter.range_truncate()?;
            });
            load_where.push(format!("end_at >= ${bind}"));
            bind += 1;
            load_where.push(format!("start_at <= ${bind}"));
            bind += 1;
            load_params.push(quote! {
                filter.#ident.as_ref().unwrap().start(),
                filter.#ident.as_ref().unwrap().end(),
            });
        } else {
            packed_fields.push(quote! { #ident: Vec<u8>, });
        }
    }
    let packed_fields = tokens(packed_fields);
    let load_checks = tokens(load_checks);
    let load_where = if load_where.is_empty() { "true".to_string() } else { load_where.join(" AND ") };
    let load_params = tokens(load_params);


    let filter = filter(model.clone(), args.clone(), using_chrono, &timestamp_ty);
    let fields = fields(model.clone(), args.clone(), packed_name.clone());
    let deserialize_time_range = timestamp_ty.map(|t| deserialize_time_range(&t));

    let load = load::generate(&packed_name, &table_name, &load_checks, &load_where, &load_params);
    let delete = delete::generate(&packed_name, &table_name, &load_checks, &load_where, &load_params);
    let decompress = decompress::generate(&model, &timestamp, &group_by, float_round, &table_name, using_chrono);
    let store_and_store_grouped = store::generate(&model, &timestamp, &group_by, float_round, &table_name, using_chrono);

    quote! {
        use serde::Deserialize as _;

        #item

        #[doc=concat!(" Generated by pco_store to store and load compressed versions of [", stringify!(#name), "]")]
        pub struct #packed_name {
            #packed_fields
        }

        impl #packed_name {
            #load

            #delete

            #decompress

            #store_and_store_grouped
        }

        #filter
        #fields
        #deserialize_time_range
    }
    .into()

}

fn copy_type(rust_type: String) -> &'static str {
    match rust_type.as_str() {
        "f32" => "FLOAT4",
        "f64" => "FLOAT8",
        "i32" => "INT4",
        "i64" => "INT8",
        "SystemTime" => "TIMESTAMPTZ",
        "String" => "TEXT",
        "Uuid" => "UUID",
        _ => panic!("unsupported copy_type {rust_type:?}"),
    }
}

fn tokens(input: Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    tokens.extend(input.into_iter());
    tokens
}
