use super::*;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemStruct, Type};

pub fn generate(
    model: &ItemStruct, timestamp: &Option<Ident>, group_by: &Vec<Ident>, float_round: Option<f32>, table_name: &str, using_chrono: bool,
) -> proc_macro2::TokenStream {
    let name = model.ident.clone();

    let mut store_fields = Vec::new();
    let mut store_types = Vec::new();
    let mut store_group = Vec::new();
    let mut store_values = Vec::new();
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
            store_fields.push(ident.to_string());
            store_types.push(Ident::new(&copy_type(quote! { #ty }.to_string()), Span::call_site()));
            store_group.push(quote! { row.#ident.clone(), });
            store_values.push(quote! { &rows[0].#ident, });
        } else if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            store_fields.push("start_at".to_string());
            store_fields.push("end_at".to_string());
            store_fields.push(ident.to_string());
            store_types.push(Ident::new("TIMESTAMPTZ", Span::call_site()));
            store_types.push(Ident::new("TIMESTAMPTZ", Span::call_site()));
            store_types.push(Ident::new("BYTEA", Span::call_site()));
            store_values.push(quote! {
                &start_at, &end_at,
                &::pco::standalone::simpler_compress(&#timestamp, ::pco::DEFAULT_COMPRESSION_LEVEL).unwrap(),
            });
        } else if is_number(&ty) {
            store_fields.push(ident.to_string());
            store_types.push(Ident::new("BYTEA", Span::call_site()));
            let expr = if round_float_field {
                quote! { (r.#ident * #float_round as #ty_original).round() as i64 }
            } else if quote! { #ty_original }.to_string() == "bool" {
                quote! { r.#ident as u16 }
            } else {
                quote! { r.#ident }
            };
            store_values.push(quote! {
                &::pco::standalone::simpler_compress(
                    &rows.iter().map(|r| #expr).collect::<Vec<_>>(), ::pco::DEFAULT_COMPRESSION_LEVEL
                )?,
            });
        } else if is_nested_number(&ty) {
            store_fields.push(ident.to_string());
            store_types.push(Ident::new("BYTEA", Span::call_site()));
            let expr = if round_float_field {
                quote! { (v * #float_round as #ty_original).round() as i64 }
            } else if quote! { #ty_original }.to_string() == "bool" {
                quote! { v as u16 }
            } else {
                quote! { v }
            };
            store_values.push(quote! {
                &pco_compress_nested(
                    rows.iter().map(|r| r.#ident.iter().map(|v| *#expr).collect::<Vec<_>>()).collect::<Vec<_>>()
                )?,
            });
        } else {
            store_fields.push(ident.to_string());
            store_types.push(Ident::new("BYTEA", Span::call_site()));
            store_values.push(quote! {
                &serde_compress(rows.iter().map(|r| r.#ident.clone()).collect::<Vec<_>>())?,
            });
        }
    }
    let store_fields = store_fields.join(", ");
    let store_types = tokens(store_types.into_iter().map(|t| quote! { tokio_postgres::types::Type::#t, }).collect());
    let store_group = tokens(store_group);
    let store_values = tokens(store_values);
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
    let store_sql = format!("COPY {table_name} ({store_fields}) FROM STDIN BINARY");

    quote! {
        /// Writes the data to disk.
        pub async fn store(db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>, rows: Vec<#name>) -> anyhow::Result<()> {
            if rows.is_empty() {
                return Ok(());
            }
            let mut grouped_rows: ahash::AHashMap<_, Vec<#name>> = ahash::AHashMap::new();
            for row in rows {
                grouped_rows.entry((#store_group)).or_default().push(row);
            }
            let sql = #store_sql;
            let types = &[#store_types];
            let stmt = db.copy_in(&db.prepare_cached(&sql).await?).await?;
            let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(stmt, types);
            futures::pin_mut!(writer);
            for rows in grouped_rows.into_values() {
                #timestamp_collect
                writer.as_mut().write(&[#store_values]).await?;
            }
            writer.finish().await?;
            Ok(())
        }

        /// Writes the data to disk, with the provided grouping closure applied.
        ///
        /// This can be used to improve the compression ratio and reduce read IO, for example
        /// by compacting real-time data into a single row per hour / day / week.
        pub async fn store_grouped<F, R>(
            db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            rows: Vec<#name>,
            grouping: F,
        ) -> anyhow::Result<()>
        where
            F: Fn(&#name) -> R,
            R: Eq + std::hash::Hash,
        {
            if rows.is_empty() {
                return Ok(());
            }
            let mut grouped_rows: ahash::AHashMap<_, Vec<#name>> = ahash::AHashMap::new();
            for row in rows {
                grouped_rows.entry((#store_group grouping(&row))).or_default().push(row);
            }
            let sql = #store_sql;
            let types = &[#store_types];
            let stmt = db.copy_in(&db.prepare_cached(&sql).await?).await?;
            let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(stmt, types);
            futures::pin_mut!(writer);
            for rows in grouped_rows.into_values() {
                #timestamp_collect
                writer.as_mut().write(&[#store_values]).await?;
            }
            writer.finish().await?;
            Ok(())
        }
    }
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
