use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemStruct, Lit, Result, Token, Type, bracketed, parse_macro_input};

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
    let Arguments { timestamp, group_by, float_round, table_name } = parse_macro_input!(a as Arguments);
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
    let mut fields = Vec::new();
    let mut load_filters = Vec::new();
    let mut load_checks = Vec::new();
    let mut load_where = Vec::new();
    let mut load_params = Vec::new();
    let mut load_fields = Vec::new();
    let mut bind = 1;
    let mut index: usize = 0;
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        if group_by.iter().any(|i| *i == ident) {
            fields.push(quote! { pub #ident: #ty, });
            load_filters.push(quote! { #ident: &[#ty], });
            let name = format!("{ident}");
            load_checks.push(quote! {
                if #ident.is_empty() {
                    anyhow::bail!(#name.to_string() + "must be specified");
                }
            });
            load_where.push(format!("{ident} = ANY(${bind})"));
            bind += 1;
            load_params.push(quote! { &#ident, });
        } else if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            fields.push(quote! {
                pub filter: bool,
                pub filter_start: SystemTime,
                pub filter_end: SystemTime,
                pub start_at: SystemTime,
                pub end_at: SystemTime,
                #ident: Vec<u8>,
            });
            load_where.push(format!("end_at >= ${bind}"));
            bind += 1;
            load_where.push(format!("start_at <= ${bind}"));
            bind += 1;
            load_params.push(quote! { &filter_start, &filter_end, });
        } else {
            fields.push(quote! { #ident: Vec<u8>, });
        }
        if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            load_fields.push(quote! { start_at: row.get(#index), });
            index += 1;
            load_fields.push(quote! { end_at: row.get(#index), });
            index += 1;
        }
        load_fields.push(quote! { #ident: row.get(#index), });
        index += 1;
    }
    let mut delete_fields = load_fields.clone();
    if timestamp.is_some() {
        load_filters.push(quote! {
            filter_start: SystemTime,
            filter_end: SystemTime,
        });
        load_fields.push(quote! { filter: true, filter_start, filter_end, });
        delete_fields.push(quote! { filter: false, filter_start, filter_end, });
    }
    let fields = tokens(fields);
    let load_filters = tokens(load_filters);
    let load_checks = tokens(load_checks);
    let load_where = if load_where.is_empty() { "true".to_string() } else { load_where.join(" AND ") };
    let load_params = tokens(load_params);
    let load_fields = tokens(load_fields);
    let delete_fields = tokens(delete_fields);

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
                decompressed_fields.push(quote! {
                    #ident: SystemTime::UNIX_EPOCH + std::time::Duration::from_micros(#ident[index]),
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
    let in_time_range = if timestamp.is_some() {
        quote! { !self.filter || row.#timestamp >= self.filter_start && row.#timestamp <= self.filter_end }
    } else {
        quote! { true }
    };

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
    let timestamp_collect = if timestamp.is_some() {
        quote! {
            let #timestamp: Vec<_> = rows.iter().map(|s| s.#timestamp).collect();
            let start_at = *#timestamp.iter().min().unwrap();
            let end_at = *#timestamp.iter().max().unwrap();
            let #timestamp: Vec<u64> = #timestamp.into_iter().map(|t|
                t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64).collect();
        }
    } else {
        quote! {}
    };

    quote! {
        #item

        #[doc=concat!(" Generated by pco_store to store and load compressed versions of [", stringify!(#name), "]")]
        pub struct #packed_name {
            #fields
        }

        impl #packed_name {
            /// Loads data for the specified filters.
            ///
            /// For models with a timestamp, [decompress][Self::decompress] automatically filters out
            /// rows outside the requested time range.
            pub async fn load(db: &deadpool_postgres::Object, #load_filters) -> anyhow::Result<Vec<#packed_name>> {
                #load_checks
                let sql = format!("SELECT * FROM {} WHERE {}", #table_name, #load_where);
                let mut results = Vec::new();
                for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                    results.push(#packed_name { #load_fields });
                }
                Ok(results)
            }

            /// Deletes data for the specified filters, returning it to the caller.
            ///
            /// For models with a timestamp, [decompress][Self::decompress] **will not** filter out
            /// rows outside the requested time range.
            pub async fn delete(db: &deadpool_postgres::Object, #load_filters) -> anyhow::Result<Vec<#packed_name>> {
                #load_checks
                let sql = format!("DELETE FROM {} WHERE {} RETURNING *", #table_name, #load_where);
                let mut results = Vec::new();
                for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                    results.push(#packed_name { #delete_fields });
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
                    if #in_time_range {
                        results.push(row);
                    }
                }
                Ok(results)
            }

            /// Writes the provided data to disk.
            pub async fn store(db: &deadpool_postgres::Object, rows: Vec<#name>) -> anyhow::Result<()> {
                if rows.is_empty() {
                    return Ok(());
                }
                let mut grouped_rows: ahash::AHashMap<_, Vec<#name>> = ahash::AHashMap::new();
                for row in rows {
                    grouped_rows.entry((#store_group)).or_default().push(row);
                }
                let sql = format!("COPY {} ({}) FROM STDIN BINARY", #table_name, #store_fields);
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
