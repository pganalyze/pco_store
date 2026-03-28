use quote::quote;
use syn::Ident;

pub fn generate(
    packed_name: &Ident, table_name: &String, load_checks: &proc_macro2::TokenStream, load_where: &String, load_params: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
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
