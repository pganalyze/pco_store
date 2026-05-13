use quote::quote;
use syn::{Ident, ItemStruct};

use super::tokens;

pub fn generate(
    model: &ItemStruct, timestamp: &Option<Ident>, group_by: &Vec<Ident>, packed_name: &Ident, table_name: &String,
) -> proc_macro2::TokenStream {
    // load and delete
    let mut load_checks = Vec::new();
    let mut load_where = Vec::new();
    let mut load_params = Vec::new();
    let mut bind = 1;
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let name = format!("{ident}");
        if group_by.iter().any(|i| *i == ident) {
            load_checks.push(quote! {
                if filter.#ident.is_empty() {
                    return Err(anyhow::Error::msg(#name.to_string() + " is required"));
                }
            });
            load_where.push(format!("{ident} = ANY(${bind})"));
            bind += 1;
            load_params.push(quote! { &filter.#ident, });
        } else if timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
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
        }
    }
    let load_checks = tokens(load_checks);
    let load_where = if load_where.is_empty() { "true".to_string() } else { load_where.join(" AND ") };
    let load_params = tokens(load_params);

    quote! {
        /// Loads data for the specified filters.
        pub async fn load(
            db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            mut filter: Filter,
            fields: impl TryInto<Fields>
        ) -> anyhow::Result<Vec<#packed_name>> {
            let mut fields = fields.try_into().map_err(|_| anyhow::Error::msg("unknown field"))?;
            fields.merge_filter(&filter);
            #load_checks
            let sql = "SELECT ".to_string() + fields.select().as_str() + " FROM " + #table_name + " WHERE " + #load_where;
            let mut results = Vec::new();
            for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                results.push(fields.load_from_row(row, Some(filter.clone()))?);
            }
            Ok(results)
        }

        /// Deletes data for the specified filters, returning it to the caller.
        ///
        /// Note that all rows are returned from [decompress][Self::decompress] even if post-decompress filters would normally apply.
        pub async fn delete(
            db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            mut filter: Filter,
            fields: impl TryInto<Fields>
        ) -> anyhow::Result<Vec<#packed_name>> {
            let mut fields = fields.try_into().map_err(|_| anyhow::Error::msg("unknown field"))?;
            fields.merge_filter(&filter);
            #load_checks
            let sql = "DELETE FROM ".to_string() + #table_name + " WHERE " + #load_where + " RETURNING " + fields.select().as_str();
            let mut results = Vec::new();
            for row in db.query(&db.prepare_cached(&sql).await?, &[#load_params]).await? {
                results.push(fields.load_from_row(row, None)?);
            }
            Ok(results)
        }
    }
}
