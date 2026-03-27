use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Lit, Result, Token, bracketed, parse_macro_input};

mod deserialize_time_range;
mod fields;
mod filter;
mod store;

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
    let model = parse_macro_input!(i as ItemStruct);
    let item = proc_macro2::TokenStream::from(item);

    store::generate(args, model, item)
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
