use quote::quote;
use syn::Ident;

pub fn generate(
    name: &Ident, store_group: &proc_macro2::TokenStream, store_sql: &str, store_types: &proc_macro2::TokenStream, timestamp_collect: &proc_macro2::TokenStream, store_values: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
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
