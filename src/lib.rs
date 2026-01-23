use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Lit, Result, Token, Type, bracketed, parse_macro_input};

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
    let mut fields = vec![quote! { filter: Option<Filter>, }];
    let mut load_checks = Vec::new();
    let mut load_where = Vec::new();
    let mut load_params = Vec::new();
    let mut load_fields = Vec::new();
    let mut bind = 1;
    let mut index: usize = 0;
    let mut timestamp_ty = None;
    let mut using_chrono = false;
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        let name = format!("{ident}");
        if group_by.iter().any(|i| *i == ident) {
            fields.push(quote! { #ident: #ty, });
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
            fields.push(quote! { #ident: Vec<u8>, });
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
            fields.push(quote! { #ident: Vec<u8>, });
        }
        if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            index += 2; // Skip over start_at and end_at
        }
        load_fields.push(quote! { #ident: row.get(#index), });
        index += 1;
    }
    let fields = tokens(fields);
    let load_checks = tokens(load_checks);
    let load_where = if load_where.is_empty() { "true".to_string() } else { load_where.join(" AND ") };
    let load_params = tokens(load_params);
    let load_fields = tokens(load_fields);
    let load_sql = format!("SELECT * FROM {table_name} WHERE {load_where}");
    let delete_sql = format!("DELETE FROM {table_name} WHERE {load_where} RETURNING *");

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
            decompressed_fields.push(quote! { #ident: self.#ident, });
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
            store_group.push(quote! { row.#ident, });
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

    let filter = filter(model, args, using_chrono, &timestamp_ty);
    let deserialize_time_range = timestamp_ty.map(|t| deserialize_time_range(&t));

    quote! {
        #item

        #[doc=concat!(" Generated by pco_store to store and load compressed versions of [", stringify!(#name), "]")]
        pub struct #packed_name {
            #fields
        }

        impl #packed_name {
            /// Loads data for the specified filters.
            pub async fn load(db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>, mut filter: Filter) -> anyhow::Result<Vec<#packed_name>> {
                #load_checks
                let sql = #load_sql;
                let mut results = Vec::new();
                for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                    results.push(#packed_name { filter: Some(filter.clone()), #load_fields });
                }
                Ok(results)
            }

            /// Deletes data for the specified filters, returning it to the caller.
            ///
            /// Note that all rows are returned from [decompress][Self::decompress] even if post-decompress filters would normally apply.
            pub async fn delete(db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>, mut filter: Filter) -> anyhow::Result<Vec<#packed_name>> {
                #load_checks
                let sql = #delete_sql;
                let mut results = Vec::new();
                for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                    results.push(#packed_name { filter: None, #load_fields });
                }
                Ok(results)
            }

            /// Decompresses a group of data points.
            pub fn decompress(self) -> anyhow::Result<Vec<#name>> {
                let mut results = Vec::new();
                #decompress_fields
                let len = [#compressed_field_sizes].into_iter().max().unwrap_or(0);
                for index in 0..len {
                    let row = #name { #decompressed_fields };
                    if self.filter.as_ref().map(|f| f.filter(&row)) != Some(false) {
                        results.push(row);
                    }
                }
                Ok(results)
            }

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
        _ => panic!("unsupported copy_type {rust_type:?}"),
    }
}

fn tokens(input: Vec<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    tokens.extend(input.into_iter());
    tokens
}

fn filter(model: ItemStruct, args: Arguments, using_chrono: bool, timestamp_ty: &Option<Type>) -> proc_macro2::TokenStream {
    let name = model.ident.clone();
    let Arguments { timestamp, group_by, .. } = args;
    let mut filter_fields = Vec::new();
    let mut filter_conditions = Vec::new();
    let mut filter_new_args = Vec::new();
    let mut filter_new_names = Vec::new();
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        let time = ty.to_token_stream().to_string().contains("Time");
        if time {
            filter_fields.push(quote! {
                #[serde(deserialize_with = "deserialize_time_range")]
                pub #ident: Option<std::ops::RangeInclusive<#ty>>,
            });
            filter_conditions.push(quote! {
                self.#ident.as_ref().map(|t| t.contains(&row.#ident)) != Some(false)
            });
        } else {
            filter_fields.push(quote! {
                #[serde(default)]
                #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
                pub #ident: Vec<#ty>,
            });
            filter_conditions.push(quote! {
                (self.#ident.is_empty() || self.#ident.contains(&row.#ident))
            });
        }
        if group_by.iter().any(|i| *i == ident) || timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            if time {
                filter_new_args.push(quote! { #ident: std::ops::RangeInclusive<#ty>, });
                filter_new_names.push(quote! { #ident: Some(#ident), });
            } else {
                filter_new_args.push(quote! { #ident: &[#ty], });
                filter_new_names.push(quote! { #ident: #ident.into(), });
            }
        }
    }
    let filter_fields = tokens(filter_fields);
    let filter_new_args = tokens(filter_new_args);
    let filter_new_names = tokens(filter_new_names);
    let timestamp_helpers = timestamp.map(|timestamp| {
        let (duration_type, duration_math) = if using_chrono {
            (quote! { chrono::Duration }, quote! { end - start })
        } else {
            (quote! { std::time::Duration }, quote! { start.duration_since(end)? })
        };
        let truncate_nanos = if using_chrono {
            quote! {
                Ok(chrono::DateTime::from_timestamp_micros(time.timestamp_micros()).context("out of range")?)
            }
        } else {
            quote! {
                use std::time::{UNIX_EPOCH, Duration};
                let duration = time.duration_since(UNIX_EPOCH).context("earlier than epoch")?;
                let micros = duration.as_secs() * 1_000_000 + (duration.subsec_nanos() / 1_000) as u64;
                Ok(UNIX_EPOCH + Duration::from_secs(micros / 1_000_000) + Duration::from_micros(micros % 1_000_000))
            }
        };
        quote! {
            /// Convenience function to unwrap the timestamp range lower and upper bounds
            pub fn range_bounds(&self) -> anyhow::Result<(#timestamp_ty, #timestamp_ty)> {
                use anyhow::Context;
                let timestamp = self.#timestamp.clone().context("no timestamp")?;
                Ok((*timestamp.start(), *timestamp.end()))
            }

            /// Convenience function to return the amount of time the filter covers
            pub fn range_duration(&self) -> anyhow::Result<#duration_type> {
                let (start, end) = self.range_bounds()?;
                Ok(#duration_math)
            }

            /// Shifts the filtered time range. This for example makes it easier
            /// to perform two queries: once for "today", and one for "today, 7 days ago".
            /// In that example the second query would do `filter.shift(Duration::days(-7))`
            pub fn range_shift(&mut self, duration: #duration_type) -> anyhow::Result<()> {
                use std::ops::Add;
                let (start, end) = self.range_bounds()?;
                self.#timestamp = Some(start.add(duration)..=end.add(duration));
                Ok(())
            }

            /// Postgres doesn't support nanosecond precision and nor does MacOS, so this
            /// truncates nanosecond precision for timestamp comparisons
            fn range_truncate(&mut self) -> anyhow::Result<()> {
                let (start, end) = self.range_bounds()?;
                self.#timestamp = Some(Self::truncate_nanos(start)?..=Self::truncate_nanos(end)?);
                Ok(())
            }

            fn truncate_nanos(time: #timestamp_ty) -> anyhow::Result<#timestamp_ty> {
                use anyhow::Context;
                #truncate_nanos
            }
        }
    });
    quote! {
        #[serde_with::serde_as]
        #[derive(Debug, Default, serde::Deserialize, Clone, PartialEq)]
        #[serde(deny_unknown_fields)]
        #[doc=concat!(" Generated by pco_store to specify filters when loading [", stringify!(#name), "]")]
        pub struct Filter {
            #filter_fields
        }

        impl Filter {
            /// Builds new filter with the required fields defined by `group_by` and `timestamp`
            pub fn new(#filter_new_args) -> Self {
                Self { #filter_new_names ..Self::default() }
            }

            fn filter(&self, row: &#name) -> bool {
                #(#filter_conditions)&&*
            }

            #timestamp_helpers
        }
    }
}

fn deserialize_time_range(timestamp_ty: &Type) -> proc_macro2::TokenStream {
    quote! {
        /// Deserializes many different time range formats:
        /// - an array with two strings becomes a normal time range: ["a", "b"] -> a..=b
        /// - an array with one string becomes a single-value time range: ["a"] -> a..=a
        /// - a string literal becomes a single-value time range:           "a" -> a..=a
        fn deserialize_time_range<'de, D>(deserializer: D) -> Result<Option<std::ops::RangeInclusive<#timestamp_ty>>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TimeRange::deserialize(deserializer)?.0)
        }

        use serde::Deserialize as _;

        #[derive(Debug, PartialEq)]
        struct TimeRange(Option<std::ops::RangeInclusive<#timestamp_ty>>);
        impl<'de> serde::Deserialize<'de> for TimeRange {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_any(TimeRangeVisitor)
            }
        }

        struct TimeRangeVisitor;
        impl<'de> serde::de::Visitor<'de> for TimeRangeVisitor {
            type Value = TimeRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a single time string or an array with 1-2 time strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value.is_empty() {
                    return Ok(TimeRange(None));
                }
                match serde::Deserialize::deserialize(serde::de::value::StrDeserializer::<E>::new(value)) {
                    Ok(start) => Ok(TimeRange(Some(start..=start))),
                    Err(err) => Err(E::custom("invalid time format: ".to_string() + err.to_string().as_str())),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let start = match seq.next_element::<Option<#timestamp_ty>>()? {
                    Some(Some(time)) => time,
                    Some(None) | None => return Ok(TimeRange(None)),
                };
                let end = match seq.next_element::<Option<#timestamp_ty>>()? {
                    Some(Some(time)) => time,
                    Some(None) | None => start,
                };
                Ok(TimeRange(Some(start..=end)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TimeRange(None))
            }
        }
    }
}
