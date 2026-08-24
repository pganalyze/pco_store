use quote::{ToTokens, quote};
use syn::{Ident, ItemStruct};

pub fn generate(model: &ItemStruct, timestamp: Option<&Ident>, index_fields: Vec<Ident>, table_name: String) -> proc_macro2::TokenStream {
    let name = model.ident.clone();
    let mut pg_columns = Vec::new();
    let mut pg_types = Vec::new();
    let mut chunk_values = Vec::new();
    for field in model.fields.iter() {
        let ident = field.ident.as_ref().unwrap();
        let name_str = ident.to_string();
        if index_fields.contains(ident) {
            pg_columns.push(name_str);
            let pg_type = map_rust_type_to_pg(field.ty.clone());
            pg_types.push(quote! { #pg_type });
            // SmolStr has no ToSql impl in tokio-postgres (only smol_str 0.1 does), so write as &str.
            if field.ty.to_token_stream().to_string().replace(" ", "").contains("SmolStr") {
                chunk_values.push(quote! { &chunk.#ident.as_str() });
            } else {
                chunk_values.push(quote! { &chunk.#ident });
            }
        } else if timestamp.map(|t| t == ident).unwrap_or(false) {
            pg_columns.extend(["start_at".to_string(), "end_at".to_string(), name_str]);
            pg_types.extend([
                quote! { tokio_postgres::types::Type::TIMESTAMPTZ },
                quote! { tokio_postgres::types::Type::TIMESTAMPTZ },
                quote! { tokio_postgres::types::Type::BYTEA },
            ]);
            chunk_values.extend([quote! { &chunk.start_at }, quote! { &chunk.end_at }, quote! { &(*chunk.#ident).to_vec() }]);
        } else {
            pg_columns.push(name_str);
            pg_types.push(quote! { tokio_postgres::types::Type::BYTEA });
            chunk_values.push(quote! { &(*chunk.#ident).to_vec() });
        }
    }
    let pg_columns = pg_columns.join(", ");
    quote! {
        /// Writes the data to disk.
        pub async fn store(
            db: &impl std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
            rows: Vec<#name>,
        ) -> anyhow::Result<()> {
            if rows.is_empty() { return Ok(()); }
            let chunks = #name::write(rows)?;
            let stmt = db.prepare_cached(&format!("COPY {} ({}) FROM STDIN BINARY", #table_name, #pg_columns)).await?;
            let types = &[#(#pg_types),*];
            let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(db.copy_in(&stmt).await?, types);
            futures::pin_mut!(writer);
            for chunk in chunks {
                writer.as_mut().write(&[#(#chunk_values),*]).await?;
            }
            writer.finish().await?;
            Ok(())
        }
    }
}

fn map_rust_type_to_pg(ty: syn::Type) -> proc_macro2::TokenStream {
    let ty_str = ty.to_token_stream().to_string();
    match ty_str.replace(" ", "").as_str() {
        "i64" => quote! { tokio_postgres::types::Type::INT8 },
        "i32" => quote! { tokio_postgres::types::Type::INT4 },
        "f64" => quote! { tokio_postgres::types::Type::FLOAT8 },
        "f32" => quote! { tokio_postgres::types::Type::FLOAT4 },
        "String" | "smol_str::SmolStr" | "SmolStr" => quote! { tokio_postgres::types::Type::TEXT },
        "uuid::Uuid" | "Uuid" => quote! { tokio_postgres::types::Type::UUID },
        _ => quote! { panic!("unsupported index field type: {:?}", std::any::type_name::<#ty>()) },
    }
}
