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
        fn pco_compress_nested<T>(nested_values: Vec<Vec<T>>) -> anyhow::Result<Vec<u8>>
        where
            T: ::pco::data_types::Number,
        {
            let mut lengths = Vec::new();
            let mut values = Vec::new();
            for vals in nested_values {
                lengths.push(vals.len() as u64);
                values.extend(vals);
            }
            let length_bytes = ::pco::standalone::simpler_compress(&lengths, ::pco::DEFAULT_COMPRESSION_LEVEL)?;
            let value_bytes = ::pco::standalone::simpler_compress(&values, ::pco::DEFAULT_COMPRESSION_LEVEL)?;
            let (length_bytes, value_bytes) = (serde_bytes::Bytes::new(&length_bytes), serde_bytes::Bytes::new(&value_bytes));
            Ok(rmp_serde::to_vec(&(length_bytes, value_bytes))?)
        }
        fn pco_decompress_nested<T>(bytes: Vec<u8>) -> anyhow::Result<Vec<Vec<T>>>
        where
            T: ::pco::data_types::Number,
        {
            let (length_bytes, value_bytes): (Vec<u8>, Vec<u8>) = rmp_serde::from_slice(&bytes)?;
            let lengths = ::pco::standalone::simple_decompress::<u64>(&length_bytes)?;
            let values = ::pco::standalone::simple_decompress::<T>(&value_bytes)?;
            let mut values = values.into_iter();
            let mut nested_values = Vec::with_capacity(lengths.len());
            for length in lengths {
                nested_values.push(values.by_ref().take(length as usize).collect::<Vec<T>>());
            }
            Ok(nested_values)
        }
    }
}
