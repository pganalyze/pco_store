use super::*;

pub fn generate() -> proc_macro2::TokenStream {
    quote! {
        fn serde_compress<T>(items: Vec<T>) -> anyhow::Result<Vec<u8>>
        where
            T: serde::Serialize,
        {
            use std::io::Write;
            let mut output = Vec::new();
            let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
            for item in items {
                rmp_serde::encode::write(&mut encoder, &item)?;
            }
            encoder.finish()?;
            Ok(output)
        }
        fn serde_decompress<T>(input: &[u8]) -> impl Iterator<Item = anyhow::Result<T>> + '_
        where
            T: for<'de> serde::Deserialize<'de> + 'static,
        {
            let decoder = match zstd::stream::read::Decoder::new(input) {
                Ok(d) => d,
                Err(e) => return Box::new(std::iter::once(Err(e.into()))) as Box<dyn Iterator<Item = _>>,
            };
            let buffered = std::io::BufReader::with_capacity(128 * 1024, decoder);
            let mut de = rmp_serde::decode::Deserializer::new(buffered);
            Box::new(std::iter::from_fn(move || match serde::Deserialize::deserialize(&mut de) {
                Ok(item) => Some(Ok(item)),
                Err(rmp_serde::decode::Error::InvalidMarkerRead(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
                Err(e) => Some(Err(e.into())),
            }))
        }
    }
}
