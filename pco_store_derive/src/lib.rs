//! Two macros per model, in the order pco_pack itself uses:
//! `#[derive(PcoStore)]` first and `#[pco_store(...)]` below it carrying the
//! settings. rustc expands attributes top to bottom, so the derive sees the
//! raw `#[pco_store(...)]` in its input and emits the `Backend` impls the
//! storage adapters run through, one per backend enabled by crate feature,
//! plus an empty `PcoStore` impl: that trait's default methods answer
//! `store`, `load`, `delete`, and `for_each` through any `&Backend`, so nothing lands on
//! the model's inherent surface. The attribute macro expands after the derive and does the one
//! job a derive cannot: grafting `#[derive(pco_pack::PcoPack)]` and a
//! matching `#[pco_pack(...)]` onto the struct, so the pco_pack knobs are
//! declared once. The macros inject no wrapper type and no names into your
//! scope, so call sites keep naming the model and the backend. The runtime
//! (backend trait, filesystem durability) lives in `pco_store` proper.

#[cfg(feature = "fs")]
mod fs_gen;
#[cfg(feature = "postgres")]
mod pg_gen;
#[cfg(feature = "postgres")]
mod sql_where;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, ItemStruct, LitInt, Result, Token, bracketed, parse_macro_input};

/// Everything `#[pco_store(...)]` carries: `name`, the prefix every backend
/// uses for this model's objects (the Postgres table, the filesystem file
/// names, future object keys), plus the pco_pack knobs the attribute
/// forwards to `#[derive(pco_pack::PcoPack)]`. Backends themselves are
/// crate features, so the whole binary agrees on the supported rungs and
/// models never repeat them.
#[derive(Default)]
struct Arguments {
    any: bool,
    name: Option<Ident>,
    timestamp: Option<Ident>,
    index: Vec<Ident>,
    float_round: Option<LitInt>,
    time_round: Option<Expr>,
    chunk_size: Option<LitInt>,
}

impl Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = Self::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            match key.to_string().as_str() {
                "name" => args.name = Some(input.parse()?),
                "timestamp" => args.timestamp = Some(input.parse()?),
                "index" => {
                    let content;
                    bracketed!(content in input);
                    args.index = content.parse_terminated(Ident::parse, Token![,])?.into_iter().collect();
                }
                // The rounding and chunk-size knobs belong to pco_pack;
                // pco_store captures the literal and forwards it untouched,
                // so pco_pack stays the single source of truth for ranges.
                "float_round" => args.float_round = Some(input.parse()?),
                "time_round" => args.time_round = Some(input.parse()?),
                "chunk_size" => args.chunk_size = Some(input.parse()?),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown pco_store key {other}; expected one of name, timestamp, index, float_round, time_round, chunk_size"),
                    ));
                }
            }
            args.any = true;
            let _: Option<Token![,]> = input.parse().ok();
        }
        if !args.any {
            return Err(input.error("pco_store needs at least one setting: name, timestamp, index, float_round, time_round or chunk_size"));
        }
        Ok(args)
    }
}

/// The impls half of the pair: an empty `PcoStore` impl, whose trait
/// defaults give the model `store`, `load`, `delete`, and `for_each`
/// through any `&Backend`, and one `Backend` impl per backend enabled by
/// crate feature. The settings arrive from the raw `#[pco_store(...)]`
/// attribute below the derive, which rustc has not expanded yet when the
/// derive runs.
#[allow(non_snake_case)]
#[proc_macro_derive(PcoStore)]
pub fn PcoStore(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ItemStruct);
    let expanded = match expand_derive(&model) {
        Ok(tokens) => tokens,
        Err(err) => return err.to_compile_error().into(),
    };
    expanded.into()
}

fn expand_derive(model: &ItemStruct) -> Result<proc_macro2::TokenStream> {
    let Some(attr) = model.attrs.iter().find(|attr| attr.path().is_ident("pco_store")) else {
        return Err(syn::Error::new_spanned(
            &model.ident,
            "add #[pco_store(...)] below #[derive(PcoStore)]: the attribute carries the storage settings and forwards them to pco_pack::PcoPack",
        ));
    };
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(attr, "pco_store needs settings: name, timestamp, index, float_round, time_round or chunk_size"));
    };
    let args = list.parse_args::<Arguments>()?;
    check_wiring(model)?;
    let settings = StorageSettings { name: args.name, timestamp: args.timestamp, index: args.index };
    let mut tokens = storage_impls(model);
    tokens.extend(backends(model, &settings)?);
    Ok(tokens)
}

/// The model-level impls every PcoStore model gets regardless of backend:
/// the trait marker; all behavior lives in the trait's defaults, and the
/// typed filter reaches users through the crate-level `pco_store::Filter<M>`
/// alias instead.
fn storage_impls(model: &ItemStruct) -> proc_macro2::TokenStream {
    let name = &model.ident;
    quote! { impl pco_store::PcoStore for #name {} }
}

/// The pco_pack macros must not collide with the emitted ones (two sources
/// of truth read worse than one clear error).
fn check_wiring(model: &ItemStruct) -> Result<()> {
    for attr in &model.attrs {
        if attr.path().is_ident("pco_pack") {
            return Err(syn::Error::new_spanned(
                attr,
                "move the #[pco_pack(...)] settings into #[pco_store(...)]; pco_store forwards them to pco_pack",
            ));
        }
        if attr.path().is_ident("derive") {
            let Ok(derives) = attr.parse_args_with(syn::punctuated::Punctuated::<syn::Path, Token![,]>::parse_terminated) else {
                continue;
            };
            for path in &derives {
                let Some(segment) = path.segments.last() else { continue };
                if segment.ident == "PcoPack" {
                    return Err(syn::Error::new_spanned(attr, "remove PcoPack from #[derive(...)]; pco_store attaches pco_pack::PcoPack itself"));
                }
            }
        }
    }
    Ok(())
}

/// The forwarding half of the pair: grafts `#[derive(pco_pack::PcoPack)]`
/// and the forwarded knobs onto the struct. Only an attribute macro can
/// rewrite the item, which is why the pair keeps this half separate; rustc
/// expands it after the derive has already read the settings. Settings are
/// mandatory: an attribute with nothing to forward would just add noise, so
/// `#[pco_store]` alone is a compile error.
#[proc_macro_attribute]
pub fn pco_store(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Arguments);
    let mut model = parse_macro_input!(item as ItemStruct);
    model.attrs.push(syn::parse_quote! { #[derive(pco_store::pco_pack::PcoPack)] });
    if let Some(knobs) = pco_pack_attr(&args) {
        model.attrs.push(syn::parse_quote! { #[pco_pack(#(#knobs)*)] });
    }
    quote! { #model }.into()
}

/// The forwarded pco_pack knobs as attribute fragments. Unset knobs are
/// dropped; pco_pack's own derive validates their values, which keeps the
/// range checks out of here.
fn pco_pack_attr(args: &Arguments) -> Option<Vec<proc_macro2::TokenStream>> {
    let mut knobs = Vec::new();
    if let Some(timestamp) = &args.timestamp {
        knobs.push(quote! { timestamp = #timestamp, });
    }
    if !args.index.is_empty() {
        let index = &args.index;
        knobs.push(quote! { index = [#(#index),*], });
    }
    if let Some(float_round) = &args.float_round {
        knobs.push(quote! { float_round = #float_round, });
    }
    if let Some(time_round) = &args.time_round {
        knobs.push(quote! { time_round = #time_round, });
    }
    if let Some(chunk_size) = &args.chunk_size {
        knobs.push(quote! { chunk_size = #chunk_size, });
    }
    (!knobs.is_empty()).then_some(knobs)
}

/// The storage half of `#[pco_store(...)]`; the packing knobs travel to
/// pco_pack instead of here.
#[derive(Default)]
struct StorageSettings {
    name: Option<Ident>,
    timestamp: Option<Ident>,
    index: Vec<Ident>,
}

fn backends(model: &ItemStruct, settings: &StorageSettings) -> Result<proc_macro2::TokenStream> {
    let name = settings.name.as_ref().map(|name| name.to_string()).unwrap_or_else(|| default_name(model));
    let mut tokens = proc_macro2::TokenStream::new();
    #[cfg(feature = "fs")]
    tokens.extend(fs_gen::generate(model, &name, &settings.index, settings.timestamp.as_ref()));
    #[cfg(feature = "postgres")]
    tokens.extend(pg_gen::generate(model, &name, &settings.index, settings.timestamp.as_ref()));
    Ok(tokens)
}

/// The default storage name: the struct name in snake_case with a plural
/// `s` (`QueryStat` becomes `query_stats`).
fn default_name(model: &ItemStruct) -> String {
    let mut name = String::new();
    for c in model.ident.to_string().chars() {
        if c.is_uppercase() && !name.is_empty() {
            name += "_";
        }
        name += &c.to_lowercase().to_string();
    }
    name += "s";
    name
}

/// Returns the last segment of a type path (e.g. `SmolStr` for `smol_str::SmolStr`). Used to match
/// the declared types of `index` fields the same way pco_pack's derive macro does.
#[cfg(any(feature = "fs", feature = "postgres"))]
fn type_last_segment(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
    {
        return Some(segment.ident.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    /// Snapshots need every backend, so the harness runs only in a build
    /// that enables them all; regenerate with
    /// `cargo test -p pco_store_derive --all-features`.
    #[cfg(all(feature = "fs", feature = "postgres"))]
    #[test]
    fn expand_snapshots() {
        use super::{Arguments, backends, check_wiring, storage_impls};
        use quote::quote;
        use std::fs;
        use std::path::Path;

        let input_dir = Path::new("tests/expand_derive");
        let files = fs::read_dir(input_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".expanded.rs"));
        for file in files {
            let input_path = file.path();
            let source = fs::read_to_string(&input_path).unwrap();
            let parsed: syn::File = syn::parse_str(&source).unwrap();
            let mut model = parsed.items.iter().find_map(|item| if let syn::Item::Struct(s) = item { Some(s.clone()) } else { None }).unwrap();

            // rustc hands an attribute macro its arguments separately and
            // strips the attribute from the item; do the same here.
            let attr = model.attrs.iter().find(|attr| attr.path().is_ident("pco_store")).expect("fixture needs a #[pco_store(...)] attribute");
            let syn::Meta::List(list) = &attr.meta else {
                panic!("fixture attribute needs arguments");
            };
            let args = list.parse_args::<Arguments>().unwrap();
            model.attrs.retain(|attr| !attr.path().is_ident("pco_store"));
            check_wiring(&model).unwrap();

            // Run both halves the way rustc would: the derive reads the raw
            // attribute and emits the impls, the attribute macro then
            // attaches the pco_pack machinery to the struct.
            let settings = super::StorageSettings { name: args.name, timestamp: args.timestamp, index: args.index };
            let storage = storage_impls(&model);
            let impls = backends(&model, &settings).unwrap();

            // Mirror the finished AST: the pco_store attribute is consumed
            // by both halves, and the pco_pack derive and knobs arrived
            // through the attribute macro.
            let mut model = model;
            model.attrs.retain(|attr| !attr.path().is_ident("derive"));
            model.attrs.push(syn::parse_quote! { #[derive(pco_store::pco_pack::PcoPack)] });
            let expanded = quote! { #model #storage #impls };
            let output = match syn::parse2::<syn::File>(expanded.clone()) {
                Ok(parsed) => prettyplease::unparse(&parsed),
                Err(_) => expanded.to_string(),
            };
            fs::write(input_path.with_extension("expanded.rs"), &output).unwrap();
        }
    }
}
