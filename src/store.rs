use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{Ident, ItemStruct, Type};

use super::deserialize_time_range::*;
use super::fields::*;
use super::filter::*;
use super::*;

mod load;
mod delete;
mod decompress;

pub fn generate(args: Arguments, model: ItemStruct, item: proc_macro2::TokenStream) -> TokenStream {
    let Arguments { timestamp, group_by, float_round, table_name } = args.clone();
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

    // decompress
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

    // store
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
        } else {
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
                ).unwrap(),
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
            let #timestamp: Vec<u64> = #timestamp.into_iter().map(|t|
                #map_inner).collect();
        }
    } else {
        quote! {}
    };
    let store_sql = format!("COPY {table_name} ({store_fields}) FROM STDIN BINARY");

    let filter = filter(model.clone(), args.clone(), using_chrono, &timestamp_ty);
    let fields = fields(model, args, packed_name.clone());
    let deserialize_time_range = timestamp_ty.map(|t| deserialize_time_range(&t));

    let load = self::load::generate(&packed_name, &table_name, &load_checks, &load_where, &load_params);
    let delete = self::delete::generate(&packed_name, &table_name, &load_checks, &load_where, &load_params);
    let decompress = self::decompress::generate(&name, &decompress_fields, &compressed_field_sizes, &decompressed_fields);

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

        #filter
        #fields
        #deserialize_time_range
    }
    .into()
}
