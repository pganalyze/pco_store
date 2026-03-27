use quote::quote;
use syn::Ident;

pub fn generate(
    name: &Ident, decompress_fields: &proc_macro2::TokenStream, compressed_field_sizes: &proc_macro2::TokenStream, decompressed_fields: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        /// Decompresses a group of data points.
        pub fn decompress(self) -> anyhow::Result<Vec<#name>> {
            let mut results = Vec::new();
            #decompress_fields
            let len = [#compressed_field_sizes].into_iter().max().unwrap_or(0);
            for index in 0..len {
                let row = #name { #decompressed_fields };
                if self.filter.as_ref().map(|f| f.matches(&row)) != Some(false) {
                    results.push(row);
                }
            }
            Ok(results)
        }
    }
}
