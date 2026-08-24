#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod load;
mod sql_where;
mod store;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, ItemStruct, Lit, Result, Token, bracketed, parse_macro_input};

#[derive(Clone, Default)]
struct Arguments {
    pub timestamp: Option<Ident>,
    pub index: Vec<Ident>,
    pub float_round: Option<u32>,
    pub time_round: Option<Expr>,
    pub chunk_size: Option<u32>,
    pub table_name: Option<String>,
}
impl Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut timestamp = None;
        let mut index = Vec::new();
        let mut float_round = None;
        let mut time_round = None;
        let mut chunk_size = None;
        let mut table_name = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            match ident.to_string().as_str() {
                "timestamp" => timestamp = Some(input.parse()?),
                "index" => {
                    let content;
                    bracketed!(content in input);
                    index = content.parse_terminated(Ident::parse, Token![,])?.into_iter().collect();
                }
                "float_round" => {
                    if let Lit::Int(value) = Lit::parse(input)? {
                        let value = value.base10_parse()?;
                        assert!(value > 0, "float_round must be greater than zero");
                        float_round = Some(value);
                    } else {
                        return Err(input.error("unsupported float_round value"));
                    }
                }
                "time_round" => time_round = Some(Expr::parse(input)?),
                "chunk_size" => {
                    if let Lit::Int(value) = Lit::parse(input)? {
                        chunk_size = Some(value.base10_parse()?);
                    } else {
                        return Err(input.error("unsupported chunk_size value"));
                    }
                }
                "table_name" => table_name = Some(input.parse::<Ident>()?.to_string()),
                _ => {
                    return Err(input.error("unexpected ident"));
                }
            }
            let _: Option<Token![,]> = input.parse().ok();
        }
        Ok(Self { timestamp, index, float_round, time_round, chunk_size, table_name })
    }
}

#[proc_macro_attribute]
/// Derives pco_pack's PcoPack trait and generates a compressed storage wrapper
/// (CompressedStructName) with store, load, and delete methods.
#[doc(hidden)]
pub fn store(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Arguments);
    let model = parse_macro_input!(item as ItemStruct);
    generate_tokens(&model, args).into()
}

fn compute_table_name(model: &ItemStruct, override_name: Option<String>) -> String {
    if let Some(name) = override_name {
        return name;
    }
    let mut table_name = String::new();
    for c in model.ident.to_string().chars() {
        if c.is_uppercase() && !table_name.is_empty() {
            table_name += "_";
        }
        table_name += &c.to_lowercase().to_string();
    }
    table_name += "s";
    table_name
}

fn generate_tokens(model: &ItemStruct, args: Arguments) -> proc_macro2::TokenStream {
    let Arguments { timestamp, index, float_round, time_round, chunk_size, table_name } = args.clone();
    let name = model.ident.clone();
    let compressed_name = Ident::new(&format!("Compressed{}s", name.to_string()), name.span());
    let table_name = compute_table_name(model, table_name);

    // Build pco_pack attribute arguments forwarded from ours
    let pco_pack_attrs = build_pco_pack_attrs(&timestamp, &index, float_round.as_ref(), time_round.as_ref(), chunk_size);

    // Remove any #[pco_store::store(...)] attributes
    let mut model_stripped = model.clone();
    model_stripped.attrs.retain(|attr| !is_pco_store_attr(attr));

    let load_and_delete = load::generate(model.clone(), timestamp.as_ref(), index.clone(), table_name.clone());
    let store = store::generate(model, timestamp.as_ref(), index.clone(), table_name);

    quote! {
        use pco_pack::PcoPack;
        #[derive(PcoPack)]
        #pco_pack_attrs
        #model_stripped

        /// Type alias for the compressed chunk representation.
        pub type Chunk = <#name as PcoPack>::Chunk;

        /// Typed filter struct provided by pco_pack.
        pub type Filter = <#name as PcoPack>::Filter;

        /// A single row of compressed data that can be decompressed on demand.
        pub struct #compressed_name {
            chunk: Chunk,
            filter: Filter,
            fields: Vec<String>,
        }

        impl std::ops::Deref for #compressed_name {
            type Target = Chunk;
            fn deref(&self) -> &Self::Target {
                &self.chunk
            }
        }

        impl #compressed_name {
            /// Decompresses this chunk using the filter and fields used to load it.
            pub fn decompress(&self) -> anyhow::Result<Vec<#name>> {
                let fields: Vec<&str> = self.fields.iter().map(|s| s.as_str()).collect();
                <#name as PcoPack>::filter(std::slice::from_ref(&self.chunk), self.filter.clone(), &fields)
            }
            #load_and_delete
            #store
        }
    }
}

fn build_pco_pack_attrs(
    timestamp: &Option<Ident>, index: &[Ident], float_round: Option<&u32>, time_round: Option<&Expr>, chunk_size: Option<u32>,
) -> proc_macro2::TokenStream {
    let mut attrs = Vec::new();
    if let Some(ts) = timestamp {
        attrs.push(quote! { timestamp = #ts });
    }
    if !index.is_empty() {
        attrs.push(quote! { index = [#(#index),*] });
    }
    if let Some(f) = float_round {
        attrs.push(quote! { float_round = #f });
    }
    if let Some(tr) = time_round {
        attrs.push(quote! { time_round = #tr });
    }
    if let Some(cs) = chunk_size {
        attrs.push(quote! { chunk_size = #cs });
    }
    quote! { #[pco_pack(#(#attrs),*)] }
}

fn is_pco_store_attr(attr: &syn::Attribute) -> bool {
    if attr.path().segments.len() < 2 {
        return false;
    }
    let crate_name = attr.path().segments.first().unwrap().ident.to_string();
    let func_name = attr.path().segments.last().unwrap().ident.to_string();
    (crate_name == "pco_store" || crate_name == "crate") && func_name == "store"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn expand_snapshots() {
        let input_dir = Path::new("tests/expand");
        let files = fs::read_dir(input_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".expanded.rs"));
        for file in files {
            let input_path = file.path();
            let input = fs::read_to_string(&input_path).unwrap();
            let file: syn::File = syn::parse_str(&input).unwrap();
            let struct_item = file.items.iter().find_map(|item| if let syn::Item::Struct(s) = item { Some(s.clone()) } else { None }).unwrap();
            // Parse the #[pco_store::store(...)] attribute
            let args = parse_store_attrs(&struct_item.attrs).unwrap();
            // Expand using the same logic as the proc_macro
            let expanded = generate_tokens(&struct_item, args);
            // Try to format with prettyplease; if it fails (e.g. references external crates), write raw tokens
            let output = match syn::parse2::<syn::File>(expanded.clone()) {
                Ok(parsed) => prettyplease::unparse(&parsed),
                Err(_) => expanded.to_string(),
            };
            let output_path = input_path.with_extension("expanded.rs");
            fs::write(&output_path, &output).unwrap();
        }
    }

    fn parse_store_attrs(attrs: &[syn::Attribute]) -> std::result::Result<Arguments, syn::Error> {
        for attr in attrs {
            if let Some(segment) = attr.path().segments.last() {
                if segment.ident == "store" {
                    return match &attr.meta {
                        syn::Meta::List(list) => list.parse_args::<Arguments>(),
                        syn::Meta::Path(_) => Ok(Default::default()), // #[pco_store::store] with no args
                        _ => Err(syn::Error::new_spanned(&attr, "unexpected attribute format")),
                    };
                }
            }
        }
        Ok(Default::default())
    }
}
