//! Postgres adapter generation: one `Backend` impl over the pool handle
//! (`pco_store::pg::Postgres`, which checks a client out per call) and one
//! over the checked-out pooled client itself
//! (`pco_store::deadpool_postgres::Object`), both running COPY, SELECT, and
//! DELETE. `index` columns stay plain so SQL keeps its teeth; BYTEA payload
//! columns hold the compressed bytes exactly as the codec wrote them.
//! Generated code names the Postgres stack through `pco_store::tokio_postgres`
//! and `pco_store::deadpool_postgres`, so only the runtime depends on it
//! directly. `sql_where` is the shared WHERE-builder core.

use quote::{ToTokens, quote};
use syn::{Field, Ident, ItemStruct};

use super::{sql_where, type_last_segment};

pub(super) fn generate(model: &ItemStruct, table: &str, index: &[Ident], timestamp: Option<&Ident>) -> proc_macro2::TokenStream {
    let named = match &model.fields {
        syn::Fields::Named(named) => named.named.clone(),
        _ => return syn::Error::new_spanned(&model.ident, "pco_store supports named fields only").to_compile_error(),
    };
    let where_builders = sql_where::generate(named.clone(), index, timestamp);
    let fields: Vec<&Field> = named.iter().collect();
    let name = &model.ident;
    let chunk = quote! { <#name as pco_store::pco_pack::PcoPack>::Chunk };
    let filter = quote! { <#name as pco_store::pco_pack::PcoPack>::Filter };
    let chunk_from_row = chunk_from_row_builder(&fields, index, timestamp, chunk.clone());

    let mut columns = Vec::new();
    let mut column_types = Vec::new();
    let mut write_values = Vec::new();
    for field in &fields {
        let ident = field.ident.as_ref().unwrap();
        let column = ident.to_string();
        if index.contains(ident) {
            columns.push(column);
            column_types.push(map_pg_type(field));
            if type_last_segment(&field.ty).as_deref() == Some("SmolStr") {
                write_values.push(quote! { &chunk.#ident.as_str() });
            } else {
                write_values.push(quote! { &chunk.#ident });
            }
        } else if timestamp == Some(ident) {
            columns.extend(["start_at".to_string(), "end_at".to_string(), column]);
            column_types.extend([
                quote! { pco_store::tokio_postgres::types::Type::TIMESTAMPTZ },
                quote! { pco_store::tokio_postgres::types::Type::TIMESTAMPTZ },
                quote! { pco_store::tokio_postgres::types::Type::BYTEA },
            ]);
            write_values.extend([quote! { &chunk.start_at }, quote! { &chunk.end_at }, quote! { &chunk.#ident.as_slice() }]);
        } else {
            columns.push(column);
            column_types.push(quote! { pco_store::tokio_postgres::types::Type::BYTEA });
            write_values.push(quote! { &chunk.#ident.as_slice() });
        }
    }
    let columns = columns.join(", ");

    let select_columns = select_column_builder(&fields, timestamp);

    // Both receivers share one body; they differ only in how they reach a
    // connection. The pool handle checks a client out per call; a
    // checked-out pooled client already *is* that connection, which keeps
    // the statement cache of `prepare_cached` on both paths.
    let backend_impl = |self_ty: proc_macro2::TokenStream, client: proc_macro2::TokenStream| {
        quote! {
            impl pco_store::Backend<#name> for #self_ty {
                fn store<'a>(&'a self, chunks: &'a [#chunk]) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<()>> {
                    Box::pin(async move {
                        if chunks.is_empty() {
                            return Ok(());
                        }
                        let db = #client;
                        let stmt = db.prepare_cached(&format!("COPY {} ({}) FROM STDIN BINARY", #table, #columns)).await?;
                        let types: &[pco_store::tokio_postgres::types::Type] = &[#(#column_types),*];
                        let mut writer = std::pin::pin!(pco_store::tokio_postgres::binary_copy::BinaryCopyInWriter::new(db.copy_in(&stmt).await?, types));
                        for chunk in chunks {
                            writer.as_mut().write(&[#(#write_values),*]).await?;
                        }
                        writer.finish().await?;
                        Ok(())
                    })
                }

                fn load<'a>(&'a self, filter: &'a #filter, fields: &'a [&str]) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<Vec<pco_store::Chunk<#name>>>> {
                    Box::pin(async move {
                        type ChunkT = #chunk;
                        let mut where_clauses: Vec<String> = Vec::new();
                        let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
                        #(#where_builders)*
                        let sql_where = if where_clauses.is_empty() {
                            String::new()
                        } else {
                            format!("WHERE {}", where_clauses.join(" AND "))
                        };
                        let filter_json: pco_store::pco_pack::serde_json::Value = filter.clone().try_into()?;
                        let fields = <#name as pco_store::pco_pack::PcoPack>::resolve_fields(&filter_json, fields)?;
                        #select_columns
                        let query_sql = format!("SELECT {} FROM {} {}", select_columns.join(", "), #table, sql_where);
                        let resolved_fields: Vec<String> = fields.iter().map(|f| (*f).to_string()).collect();
                        let db = #client;
                        let mut chunks = Vec::new();
                        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                            let mut index = 0;
                            chunks.push(pco_store::Chunk {
                                chunk: #chunk_from_row,
                                filter: filter.clone(),
                                fields: resolved_fields.clone(),
                            });
                        }
                        Ok(chunks)
                    })
                }

                /// Deletes matching rows and returns whole chunks with empty
                /// filters, so decompress answers every row the delete touched and
                /// a re-store loses nothing outside the filter range.
                fn delete<'a>(&'a self, filter: &'a #filter) -> pco_store::BoxFuture<'a, pco_store::anyhow::Result<Vec<pco_store::Chunk<#name>>>> {
                    Box::pin(async move {
                        type ChunkT = #chunk;
                        let mut where_clauses: Vec<String> = Vec::new();
                        let mut params: Vec<&(dyn pco_store::tokio_postgres::types::ToSql + Sync)> = Vec::new();
                        #(#where_builders)*
                        let sql_where = if where_clauses.is_empty() {
                            String::new()
                        } else {
                            format!("WHERE {}", where_clauses.join(" AND "))
                        };
                        let filter_json: pco_store::pco_pack::serde_json::Value = filter.clone().try_into()?;
                        let fields = <#name as pco_store::pco_pack::PcoPack>::resolve_fields(&filter_json, &[])?;
                        #select_columns
                        let query_sql = format!("DELETE FROM {} {} RETURNING {}", #table, sql_where, select_columns.join(", "));
                        let resolved_fields: Vec<String> = fields.iter().map(|f| (*f).to_string()).collect();
                        let db = #client;
                        let mut chunks = Vec::new();
                        for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                            let mut index = 0;
                            chunks.push(pco_store::Chunk {
                                chunk: #chunk_from_row,
                                filter: Default::default(),
                                fields: resolved_fields.clone(),
                            });
                        }
                        Ok(chunks)
                    })
                }

                fn capabilities(&self) -> pco_store::Capabilities {
                    pco_store::Capabilities { transactions: false, rename: false, range_get: false, sql_pushdown: true }
                }
            }
        }
    };
    let handle = backend_impl(quote! { pco_store::pg::Postgres }, quote! { self.client().await? });
    let pooled_client = backend_impl(quote! { pco_store::deadpool_postgres::Object }, quote! { self });
    quote! { #handle #pooled_client }
}

fn select_column_builder(fields: &[&Field], timestamp: Option<&Ident>) -> proc_macro2::TokenStream {
    let mut col_names = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        if timestamp == Some(ident) {
            col_names.push("start_at".to_string());
            col_names.push("end_at".to_string());
        }
        col_names.push(ident.to_string());
    }
    quote! {
        let known: &[&str] = &[#(#col_names),*];
        let select_columns: Vec<&str> = known.iter().copied().filter(|c| fields.iter().any(|f| f == c)).collect();
    }
}

fn chunk_from_row_builder(
    fields: &[&Field], index: &[Ident], timestamp: Option<&Ident>, _chunk: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut parts = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        let field_name = ident.to_string();
        if timestamp == Some(ident) {
            parts.push(quote! {
                start_at: if fields.contains(&"start_at") {
                    let v = row.get::<_, pco_store::pco_pack::chrono::DateTime<pco_store::pco_pack::chrono::Utc>>(index);
                    index += 1;
                    v
                } else {
                    Default::default()
                },
                end_at: if fields.contains(&"end_at") {
                    let v = row.get::<_, pco_store::pco_pack::chrono::DateTime<pco_store::pco_pack::chrono::Utc>>(index);
                    index += 1;
                    v
                } else {
                    Default::default()
                },
                #ident: if fields.contains(&#field_name) {
                    let v = row.get::<_, Option<&[u8]>>(index);
                    index += 1;
                    match v {
                        Some(b) => pco_store::pco_pack::serde_bytes::ByteBuf::from(b),
                        None => Default::default(),
                    }
                } else {
                    Default::default()
                },
            });
        } else if index.contains(ident) {
            let ty = &field.ty;
            if type_last_segment(&field.ty).as_deref() == Some("SmolStr") {
                parts.push(quote! {
                    #ident: if fields.contains(&#field_name) {
                        let v = row.get::<_, String>(index);
                        index += 1;
                        <#ty>::from(v)
                    } else {
                        Default::default()
                    },
                });
            } else {
                parts.push(quote! {
                    #ident: if fields.contains(&#field_name) {
                        let v = row.get::<_, #ty>(index);
                        index += 1;
                        v
                    } else {
                        Default::default()
                    },
                });
            }
        } else {
            parts.push(quote! {
                #ident: if fields.contains(&#field_name) {
                    let v = row.get::<_, Option<&[u8]>>(index);
                    index += 1;
                    match v {
                        Some(b) => pco_store::pco_pack::serde_bytes::ByteBuf::from(b),
                        None => Default::default(),
                    }
                } else {
                    Default::default()
                },
            });
        }
    }
    quote! {
        ChunkT {
            #(#parts)*
        }
    }
}

fn map_pg_type(field: &Field) -> proc_macro2::TokenStream {
    match type_last_segment(&field.ty).as_deref() {
        Some("i64") => quote! { pco_store::tokio_postgres::types::Type::INT8 },
        Some("i32") => quote! { pco_store::tokio_postgres::types::Type::INT4 },
        Some("String") | Some("SmolStr") => quote! { pco_store::tokio_postgres::types::Type::TEXT },
        Some("Uuid") => quote! { pco_store::tokio_postgres::types::Type::UUID },
        Some("bool") => quote! { pco_store::tokio_postgres::types::Type::BOOL },
        _ => panic!("unsupported index field type: {}; supported types are i64, i32, String, SmolStr, Uuid, and bool", field.ty.to_token_stream()),
    }
}
