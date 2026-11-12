//! Filesystem adapter generation: one `Backend` impl over
//! `pco_store::fs::Fs`. The chunk file name carries the table, the sanitized
//! `index` key, and the timestamp bounds, so `load` narrows by readdir and
//! name parsing before reading any bytes; the whole-chunk codec sits between
//! encoding and the durable directory.
//!
//! Name layout, pinned here: `{table}~{key}~{bounds}~{digest}.bin`. `~` and
//! `|` never appear inside a sanitized value, so segments split cleanly;
//! `{bounds}` is `{start_us}.{end_us}`, empty when the model has no
//! timestamp; `{digest}` is the hash of the encoded chunk bytes, so two
//! different chunks never overwrite one another and a duplicate store
//! overwrites itself with bytes that open to the same rows.
//!
//! The narrowing check is emitted as a local closure inside `load` and
//! `delete`, never as a named item: a derive may only add trait impls, and a
//! named inherent method on the shared `Fs` would collide when a second model
//! in the same crate picks the same adapter.

use quote::quote;
use syn::{Field, Ident, ItemStruct};

use super::type_last_segment;

pub(super) fn generate(model: &ItemStruct, table: &str, index: &[Ident], timestamp: Option<&Ident>) -> proc_macro2::TokenStream {
    let name = &model.ident;
    let chunk = quote! { <#name as pco_store::pco_pack::PcoPack>::Chunk };
    let filter = quote! { <#name as pco_store::pco_pack::PcoPack>::Filter };

    let key_parts = index.iter().map(|field| quote! { key_parts.push(pco_store::fs::sanitize(&chunk.#field.to_string())); });

    let bounds_expr = match timestamp {
        // Chunk bounds columns are named start_at/end_at by pco_pack's
        // derive, whatever the model calls its timestamp field.
        Some(_) => quote! { format!("{}.{}", chunk.start_at.timestamp_micros(), chunk.end_at.timestamp_micros()) },
        None => quote! { String::new() },
    };

    let narrow = narrowing_closure(table, &filter, &fields_of(model), index, timestamp);

    let all_columns = model.fields.iter().map(|field| {
        let column = field.ident.as_ref().unwrap().to_string();
        quote! { #column.to_string() }
    });

    quote! {
        impl pco_store::Backend<#name> for pco_store::fs::Fs {
            fn store<'a>(&'a self, chunks: &'a [#chunk]) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<()>> {
                Box::pin(async move {
                    for chunk in chunks {
                        let bytes = pco_store::encode_chunk::<#name>(chunk)?;
                        let mut key_parts: Vec<String> = Vec::new();
                        #(#key_parts)*
                        let bounds = #bounds_expr;
                        let name = format!("{}~{}~{}~{}.bin", #table, key_parts.join("|"), bounds, pco_store::fs::fnv1a_hex(&bytes));
                        self.dur().put(&name, &bytes)?;
                    }
                    Ok(())
                })
            }

            fn load<'a>(&'a self, filter: &'a #filter, fields: &'a [&str]) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<Vec<pco_store::Chunk<#name>>>> {
                Box::pin(async move {
                    let filter_json: pco_store::pco_pack::serde_json::Value = filter.clone().try_into()?;
                    let resolved = <#name as pco_store::pco_pack::PcoPack>::resolve_fields(&filter_json, fields)?;
                    let resolved_fields: Vec<String> = resolved.iter().map(|f| (*f).to_string()).collect();
                    #narrow
                    let mut chunks = Vec::new();
                    for found in self.dur().list_names()? {
                        let name = found.to_string_lossy().into_owned();
                        if !narrows(&name, filter) {
                            continue;
                        }
                        let Some(bytes) = self.dur().get(&name)? else { continue };
                        let chunk = pco_store::decode_chunk::<#name>(&bytes)?;
                        chunks.push(pco_store::Chunk { chunk, filter: filter.clone(), fields: resolved_fields.clone() });
                    }
                    Ok(chunks)
                })
            }

            fn delete<'a>(&'a self, filter: &'a #filter) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<Vec<pco_store::Chunk<#name>>>> {
                Box::pin(async move {
                    #narrow
                    let mut chunks = Vec::new();
                    for found in self.dur().list_names()? {
                        let name = found.to_string_lossy().into_owned();
                        if !narrows(&name, filter) {
                            continue;
                        }
                        let Some(bytes) = self.dur().get(&name)? else { continue };
                        let chunk = pco_store::decode_chunk::<#name>(&bytes)?;
                        self.dur().remove(&name)?;
                        chunks.push(pco_store::Chunk { chunk, filter: Default::default(), fields: vec![#(#all_columns),*] });
                    }
                    Ok(chunks)
                })
            }

            fn capabilities(&self) -> pco_store::Capabilities {
                pco_store::Capabilities { transactions: false, rename: true, range_get: false, sql_pushdown: false }
            }
        }
    }
}

/// The name-time narrowing closure for the table. The key and bounds inside
/// the file name answer before any chunk bytes are read; everything
/// unprovable from a name stays kept, because the filter applies again
/// exactly at decompression, and a chunk is dropped only when its name proves
/// it cannot match. Numeric index kinds compare numerically after parse-back;
/// text kinds compare the sanitized form directly, the same string the name
/// was built from, so equal values always match.
fn narrowing_closure(
    table: &str, filter: &proc_macro2::TokenStream, fields: &[&Field], index: &[Ident], timestamp: Option<&Ident>,
) -> proc_macro2::TokenStream {
    let narrow_checks = index.iter().enumerate().map(|(pos, ident)| narrow_check(field_of(fields, ident), pos));

    let bounds_check = timestamp.map(|ts| {
        quote! {
            if let Some(f) = filter.#ts.as_ref() {
                if let Some((head, tail)) = bounds.split_once('.') {
                    if let (Ok(chunk_start), Ok(chunk_end)) = (head.parse::<i64>(), tail.parse::<i64>()) {
                        match f {
                            pco_store::pco_pack::TimeFilter::Equal(v) => {
                                let at = v.timestamp_micros();
                                if chunk_end < at || chunk_start > at {
                                    return false;
                                }
                            }
                            pco_store::pco_pack::TimeFilter::Inclusion(vals) => {
                                if let (Some(first), Some(last)) = (vals.first(), vals.last()) {
                                    if chunk_end < first.timestamp_micros() || chunk_start > last.timestamp_micros() {
                                        return false;
                                    }
                                }
                            }
                            pco_store::pco_pack::TimeFilter::Range { start, end } => {
                                if chunk_end < start.timestamp_micros() || chunk_start > end.timestamp_micros() {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    quote! {
        let narrows = |name: &str, filter: &#filter| -> bool {
            let Some(rest) = name.strip_prefix(concat!(#table, "~")) else { return false };
            let mut segments = rest.splitn(3, '~');
            let (Some(keys), Some(bounds), Some(digest)) = (segments.next(), segments.next(), segments.next()) else { return false };
            if digest.is_empty() {
                return false;
            }
            let keys: Vec<&str> = if keys.is_empty() { Vec::new() } else { keys.split('|').collect() };
            #(#narrow_checks)*
            #bounds_check
            true
        };
    }
}

/// One generated name-match check for one `index` field at `pos` in the key.
fn narrow_check(field: &Field, pos: usize) -> proc_macro2::TokenStream {
    let ident = field.ident.as_ref().expect("named field");
    let arms = match type_last_segment(&field.ty).as_deref() {
        Some("i64") | Some("i32") => quote! {
            match f {
                pco_store::pco_pack::I64Filter::Equal(v) => {
                    if let Ok(parsed) = k.parse::<i64>() {
                        if parsed != *v {
                            return false;
                        }
                    }
                }
                pco_store::pco_pack::I64Filter::Inclusion(vals) => {
                    if !vals.iter().any(|v| k.parse::<i64>().map(|p| p == *v).unwrap_or(true)) {
                        return false;
                    }
                }
                pco_store::pco_pack::I64Filter::Range { start, end } => {
                    if let Ok(parsed) = k.parse::<i64>() {
                        if parsed < *start || parsed > *end {
                            return false;
                        }
                    }
                }
            }
        },
        Some("f64") | Some("f32") => quote! {
            match f {
                pco_store::pco_pack::F64Filter::Equal(v) => {
                    if k != pco_store::fs::sanitize(&v.to_string()).as_str() {
                        return false;
                    }
                }
                pco_store::pco_pack::F64Filter::Inclusion(vals) => {
                    if !vals.iter().any(|v| k == pco_store::fs::sanitize(&v.to_string())) {
                        return false;
                    }
                }
                pco_store::pco_pack::F64Filter::Range { .. } => {}
            }
        },
        Some("String") | Some("SmolStr") => quote! {
            match f {
                pco_store::pco_pack::StringFilter::Equal(v) => {
                    if k != pco_store::fs::sanitize(v).as_str() {
                        return false;
                    }
                }
                pco_store::pco_pack::StringFilter::Inclusion(vals) => {
                    if !vals.iter().any(|v| k == pco_store::fs::sanitize(v)) {
                        return false;
                    }
                }
            }
        },
        Some("Uuid") => quote! {
            match f {
                pco_store::pco_pack::UuidFilter::Equal(v) => {
                    if k != pco_store::fs::sanitize(&v.to_string()).as_str() {
                        return false;
                    }
                }
                pco_store::pco_pack::UuidFilter::Inclusion(vals) => {
                    if !vals.iter().any(|v| k == pco_store::fs::sanitize(&v.to_string())) {
                        return false;
                    }
                }
            }
        },
        Some("bool") => quote! {
            match f {
                pco_store::pco_pack::BoolFilter::Equal(v) => {
                    if k != pco_store::fs::sanitize(&v.to_string()).as_str() {
                        return false;
                    }
                }
                pco_store::pco_pack::BoolFilter::Inclusion(vals) => {
                    if !vals.iter().any(|v| k == pco_store::fs::sanitize(&v.to_string())) {
                        return false;
                    }
                }
            }
        },
        _ => quote! {},
    };
    quote! {
        if let (Some(f), Some(k)) = (filter.#ident.as_ref(), keys.get(#pos).copied()) {
            #arms
        }
    }
}

fn fields_of(model: &ItemStruct) -> Vec<&Field> {
    model.fields.iter().collect()
}

fn field_of<'a>(fields: &[&'a Field], ident: &Ident) -> &'a Field {
    fields.iter().find(|field| field.ident.as_ref() == Some(ident)).expect("index field exists on the model")
}
