use super::*;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::ItemStruct;

pub fn generate(model: ItemStruct, timestamp: Option<&Ident>, index_fields: Vec<Ident>, table_name: String) -> TokenStream {
    let fields = match model.fields {
        syn::Fields::Named(fields) => fields.named,
        _ => return syn::Error::new_spanned(&model.ident, "Only named struct fields are supported").to_compile_error(),
    };
    let chunk_builder = build_chunk_from_row(&fields, timestamp, &index_fields);
    let select_columns = build_select_columns(&fields, timestamp);
    let model_name = &model.ident;
    let where_builders = sql_where::generate(fields, &index_fields, timestamp);
    let load_impl = generate_load_impl(&where_builders, &select_columns, &chunk_builder, &table_name, model_name);
    let delete_impl = generate_delete_impl(&where_builders, &select_columns, &chunk_builder, &table_name, model_name);
    quote! {
        #load_impl
        #delete_impl
    }
}

fn generate_load_impl(
    where_builders: &[TokenStream], select_columns: &TokenStream, chunk_builder: &TokenStream, table_name: &String, model_name: &Ident,
) -> TokenStream {
    quote! {
        /// Loads data for the specified filters.
        pub async fn load(
            db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            filter: Filter,
            fields: &[&str],
        ) -> anyhow::Result<Vec<Self>> {
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
            #(#where_builders)*
            let sql_where = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };
            let filter_json: serde_json::Value = filter.clone().try_into()?;
            let fields = <#model_name as PcoPack>::resolve_fields(&filter_json, fields)?;
            #select_columns
            let query_sql = format!("SELECT {} FROM {} {}", select_columns.join(", "), #table_name, sql_where);
            let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
            let mut chunks: Vec<Self> = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks.push(Self {
                    chunk: #chunk_builder,
                    filter: filter.clone(),
                    fields: fields_vec.clone(),
                });
            }
            Ok(chunks)
        }
    }
}

fn generate_delete_impl(
    where_builders: &[TokenStream], select_columns: &TokenStream, chunk_builder: &TokenStream, table_name: &String, model_name: &Ident,
) -> TokenStream {
    quote! {
        /// Deletes data matching the specified filters and returns the deleted rows.
        pub async fn delete(
            db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            filter: Filter, fields: &[&str],
        ) -> anyhow::Result<Vec<Self>> {
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
            #(#where_builders)*
            let sql_where = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };
            let filter_json: serde_json::Value = filter.clone().try_into()?;
            let fields = <#model_name as PcoPack>::resolve_fields(&filter_json, fields)?;
            #select_columns
            let query_sql = format!("DELETE FROM {} {} RETURNING {}", #table_name, sql_where, select_columns.join(", "));
            let fields_vec: Vec<String> = fields.iter().map(|s| (*s).to_string()).collect();
            let mut chunks: Vec<Self> = Vec::new();
            for row in db.query(&db.prepare_cached(&query_sql).await?, &params).await? {
                let mut index = 0;
                chunks.push(Self {
                    chunk: #chunk_builder,
                    filter: filter.clone(),
                    fields: fields_vec.clone(),
                });
            }
            Ok(chunks)
        }
    }
}

fn build_select_columns(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, timestamp: Option<&proc_macro2::Ident>) -> TokenStream {
    let mut col_names = Vec::new();
    for field in fields.iter() {
        let ident = field.ident.as_ref().unwrap();
        let field_name = ident.to_string();
        if timestamp.map(|t| t == ident).unwrap_or(false) {
            col_names.push("start_at".to_string());
            col_names.push("end_at".to_string());
            col_names.push(field_name);
        } else {
            col_names.push(field_name);
        }
    }
    quote! {
        let known: &[&str] = &[#(#col_names),*];
        let select_columns: Vec<_> = known.iter().filter(|c| fields.contains(*c)).copied().collect();
    }
}

fn build_chunk_from_row(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, timestamp: Option<&proc_macro2::Ident>, index_fields: &[Ident],
) -> TokenStream {
    let mut parts = Vec::new();
    for field in fields.iter() {
        let ident = field.ident.as_ref().unwrap();
        let field_name = ident.to_string();
        if index_fields.contains(ident) {
            let ty = &field.ty;
            // SmolStr has no FromSql impl in tokio-postgres (only smol_str 0.1 does), so read as String and convert.
            if ty.to_token_stream().to_string().replace(" ", "").contains("SmolStr") {
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
                        let v = row.get(index);
                        index += 1;
                        v
                    } else {
                        Default::default()
                    },
                });
            }
        } else if timestamp.map(|t| t == ident).unwrap_or(false) {
            parts.push(quote! {
                start_at: if fields.contains(&"start_at") {
                    let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                    index += 1;
                    v
                } else {
                    Default::default()
                },
                end_at: if fields.contains(&"end_at") {
                    let v = row.get::<_, chrono::DateTime<chrono::Utc>>(index);
                    index += 1;
                    v
                } else {
                    Default::default()
                },
                #ident: if fields.contains(&#field_name) {
                    let v = row.get::<_, Option<&[u8]>>(index);
                    index += 1;
                    v.map_or_else(Default::default, pco_pack::serde_bytes::ByteBuf::from)
                } else {
                    Default::default()
                },
            });
        } else {
            parts.push(quote! {
                #ident: if fields.contains(&#field_name) {
                    let v = row.get::<_, Option<&[u8]>>(index);
                    index += 1;
                    v.map_or_else(Default::default, pco_pack::serde_bytes::ByteBuf::from)
                } else {
                    Default::default()
                },
            });
        }
    }
    quote! {
        Chunk { #(#parts)* }
    }
}
