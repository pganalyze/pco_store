use quote::quote;
use syn::Ident;

pub fn generate(
    packed_name: &Ident, table_name: &String, load_checks: &proc_macro2::TokenStream, load_where: &String, load_params: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
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
    }
}
