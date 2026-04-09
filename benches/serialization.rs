use anyhow::Result;
use indexmap::IndexMap;
use peak_alloc::PeakAlloc;
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::time::Instant;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn main() {
    println!();
    bench_pco(numbers(), "i64");
    bench_serde(numbers(), "i64");
    bench_facet(numbers(), "i64");
    println!();
    bench_flat_pco(numbers_nested(), "Vec<u64> + Vec<i64> (flattened)");
    bench_serde(numbers_nested(), "Vec<i64>");
    bench_facet(numbers_nested(), "Vec<i64>");
    println!();
    bench_serde(strings(), "String");
    bench_facet(strings(), "String");
    println!();
    bench_serde(serde_values(), "serde_json::Value");
    bench_facet(facet_values(), "facet_value::Value");
    println!();
    bench_serde(hashmap(), "HashMap<String, String>");
    bench_facet(hashmap(), "HashMap<String, String>");
    println!();
    bench_serde(btreemap(), "BTreeMap<String, String>");
    bench_facet(btreemap(), "BTreeMap<String, String>");
    println!();
    bench_serde(indexmap(), "IndexMap<String, String>");
    println!();
    bench_serde(structs(), "Struct");
    bench_facet(structs(), "Struct");
    println!();
}

fn bench_pco<T>(values: Vec<T>, r#type: &str)
where
    T: pco::data_types::Number,
{
    println!("== pco Vec<{type}>");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let bytes = pco::standalone::simpler_compress(&values, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let result: Vec<T> = pco::standalone::simple_decompress(&bytes).unwrap();
    black_box(result);
    println!("deserialized and retained after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
}

fn bench_flat_pco<T>(nested_values: Vec<Vec<T>>, r#type: &str)
where
    T: pco::data_types::Number,
{
    let mut lengths = Vec::new();
    let mut values = Vec::new();
    for value in nested_values {
        lengths.push(value.len() as u64);
        values.extend(value);
    }
    println!("== pco {type}");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let length_bytes = pco::standalone::simpler_compress(&lengths, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let value_bytes = pco::standalone::simpler_compress(&values, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let bytes = encode_nested_vec(length_bytes, value_bytes);
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let (length_bytes, value_bytes) = decode_nested_vec(&bytes).unwrap();
    let lengths = pco::standalone::simple_decompress::<u64>(&length_bytes).unwrap();
    let values = pco::standalone::simple_decompress::<T>(&value_bytes).unwrap();
    let mut values = values.into_iter();
    let mut combined = Vec::with_capacity(lengths.len());
    for length in lengths {
        combined.push(values.by_ref().take(length as usize).collect::<Vec<T>>());
    }
    black_box(combined);
    println!("deserialized and retained after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
}

fn encode_nested_vec(v1: Vec<u8>, v2: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + v1.len() + 8 + v2.len());
    out.extend_from_slice(&(v1.len() as u64).to_le_bytes());
    out.extend(v1);
    out.extend_from_slice(&(v2.len() as u64).to_le_bytes());
    out.extend(v2);
    out
}

fn decode_nested_vec(data: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let len1 = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
    let vec1 = &data[8..8 + len1];
    let len2 = u64::from_le_bytes(data[8 + len1..16 + len1].try_into().unwrap()) as usize;
    let vec2 = &data[16 + len1..16 + len1 + len2];
    Ok((vec1, vec2))
}

fn bench_serde<T>(values: Vec<T>, r#type: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    println!("== serde Vec<{type}>");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let bytes = serde_compress(values).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    for v in serde_decompress::<T>(&bytes) {
        black_box(v.unwrap());
    }
    println!("deserialized and discarded after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let result: Vec<T> = serde_decompress::<T>(&bytes).collect::<Result<_>>().unwrap();
    black_box(result);
    println!("deserialized and retained after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
}

fn bench_facet<T>(values: Vec<T>, r#type: &str)
where
    T: for<'a> facet::Facet<'a>,
{
    println!("== facet_postcard Vec<{type}>");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let bytes = facet_compress(values).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    for v in facet_decompress::<T>(&bytes) {
        black_box(v.unwrap());
    }
    println!("deserialized and discarded after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let result: Vec<T> = facet_decompress::<T>(&bytes).collect::<Result<_>>().unwrap();
    black_box(result);
    println!("deserialized and retained after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
}

fn strings() -> Vec<String> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.extend([format!("Key A {i}"), format!("Value A {i}"), format!("Key B {id}"), format!("Value B {id}")]);
    }
    values
}

fn numbers() -> Vec<u64> {
    let mut values = Vec::new();
    for i in 0..100_000u64 {
        values.extend([i, i * 2, i.pow(2)]);
    }
    values
}

fn numbers_nested() -> Vec<Vec<u64>> {
    let mut values = Vec::new();
    for i in 0..100_000u64 {
        values.push(vec![i, i * 2, i.pow(2)]);
    }
    values
}

fn serde_values() -> Vec<serde_json::Value> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.push(serde_json::json!({
            format!("Key A {i}"): format!("Value A {i}"),
            format!("Key B {id}"): format!("Value B {id}"),
        }));
    }
    values
}

fn facet_values() -> Vec<facet_value::Value> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        let (keya, valuea) = (format!("Key A {i}"), format!("Value A {i}"));
        let (keyb, valueb) = (format!("Key B {id}"), format!("Value B {id}"));
        values.push(facet_value::value!({ keya: valuea, keyb: valueb }));
    }
    values
}

fn hashmap() -> Vec<HashMap<String, String>> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.push(HashMap::from([(format!("Key A {i}"), format!("Value A {i}")), (format!("Key B {id}"), format!("Value B {id}"))]));
    }
    values
}

fn btreemap() -> Vec<BTreeMap<String, String>> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.push(BTreeMap::from([(format!("Key A {i}"), format!("Value A {i}")), (format!("Key B {id}"), format!("Value B {id}"))]));
    }
    values
}

fn indexmap() -> Vec<IndexMap<String, String>> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.push(IndexMap::from([(format!("Key A {i}"), format!("Value A {i}")), (format!("Key B {id}"), format!("Value B {id}"))]));
    }
    values
}

#[derive(serde::Serialize, serde::Deserialize, facet::Facet)]
struct Struct {
    a: u32,
    b: u32,
    c: String,
    d: String,
}

fn structs() -> Vec<Struct> {
    let mut values = Vec::new();
    for i in 0..100_000u32 {
        let id = i.pow(2);
        values.push(Struct { a: i, b: id, c: format!("String {i}"), d: format!("Other {id}") });
    }
    values
}

//
// Below is custom compression + serialization logic for serde and facet_postcard.
// Since string-like data can be very large, we use a streaming approach so during decompression,
// each row can be individually hydrated into memory, significantly reducing memory usage
// when many rows are immediately filtered out.
//

fn serde_compress<T>(items: Vec<T>) -> anyhow::Result<Vec<u8>>
where
    T: serde::Serialize,
{
    use std::io::Write;
    let mut output = Vec::new();
    let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
    for item in items {
        serde_json::to_writer(&mut encoder, &item)?;
        encoder.write_all(b"\n")?; // The "L" in JSONL
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
    let json_iter = serde_json::Deserializer::from_reader(buffered).into_iter::<T>();
    Box::new(json_iter.map(|res| res.map_err(anyhow::Error::from)))
}

fn facet_compress<T, I>(items: I) -> anyhow::Result<Vec<u8>>
where
    T: for<'a> facet::Facet<'a>,
    I: IntoIterator<Item = T>,
{
    use std::io::Write;
    let mut output = Vec::new();
    let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
    for item in items {
        let bytes = facet_postcard::to_vec(&item)?;
        // Encode and write the length prefix
        let mut len = bytes.len();
        loop {
            let byte = (len & 0x7F) as u8;
            len >>= 7;
            if len > 0 {
                // Set continuation bit
                encoder.write_all(&[byte | 0x80])?;
            } else {
                encoder.write_all(&[byte])?;
                break;
            }
        }
        // Write the actual data
        encoder.write_all(&bytes)?;
    }
    encoder.finish()?;
    Ok(output)
}

fn facet_decompress<'a, T>(input: &'a [u8]) -> impl Iterator<Item = anyhow::Result<T>> + 'a
where
    T: facet::Facet<'static> + 'a,
{
    use std::io::Read;
    let decoder = zstd::stream::read::Decoder::new(input).unwrap();
    let mut reader = std::io::BufReader::with_capacity(128 * 1024, decoder);
    let mut buffer = Vec::new();
    std::iter::from_fn(move || {
        // Read length prefix
        let mut len: usize = 0;
        let mut shift = 0;
        loop {
            let mut b = [0u8; 1];
            if let Err(_) = reader.read_exact(&mut b) {
                return None; // EOF
            }
            let byte = b[0];
            len |= ((byte & 0x7F) as usize) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift > 63 {
                return Some(Err(anyhow::anyhow!("Varint overflow")));
            }
        }
        // Read the exact number of bytes from the length prefix
        buffer.resize(len, 0);
        if let Err(e) = reader.read_exact(&mut buffer) {
            return Some(Err(e.into()));
        }
        match facet_postcard::from_slice::<T>(&buffer) {
            Ok(item) => Some(Ok(item)),
            Err(e) => Some(Err(anyhow::anyhow!("Postcard error: {:?}", e))),
        }
    })
}
