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
    bench_msgpack(numbers(), "i64");
    bench_cbor(numbers(), "i64");
    println!();
    bench_flat_serde(numbers_nested(), "Vec<u64> + Vec<i64> (flattened)");
    bench_flat_msgpack(numbers_nested(), "Vec<u64> + Vec<i64> (flattened)");
    bench_flat_cbor(numbers_nested(), "Vec<u64> + Vec<i64> (flattened)");
    bench_serde(numbers_nested(), "Vec<i64>");
    bench_msgpack(numbers_nested(), "Vec<i64>");
    bench_cbor(numbers_nested(), "Vec<i64>");
    println!();
    bench_serde(strings(), "String");
    bench_msgpack(strings(), "String");
    bench_cbor(strings(), "String");
    println!();
    bench_serde(serde_values(), "serde_json::Value");
    bench_msgpack(serde_values(), "serde_json::Value");
    bench_cbor(serde_values(), "serde_json::Value");
    println!();
    bench_serde(hashmap(), "HashMap<String, String>");
    bench_msgpack(hashmap(), "HashMap<String, String>");
    bench_cbor(hashmap(), "HashMap<String, String>");
    println!();
    bench_serde(btreemap(), "BTreeMap<String, String>");
    bench_msgpack(btreemap(), "BTreeMap<String, String>");
    bench_cbor(btreemap(), "BTreeMap<String, String>");
    println!();
    bench_serde(indexmap(), "IndexMap<String, String>");
    bench_msgpack(indexmap(), "IndexMap<String, String>");
    bench_cbor(indexmap(), "IndexMap<String, String>");
    println!();
    bench_serde(structs(), "Struct");
    bench_msgpack(structs(), "Struct");
    bench_cbor(structs(), "Struct");
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

fn bench_flat_serde<T>(nested_values: Vec<Vec<T>>, r#type: &str)
where
    T: pco::data_types::Number,
{
    let mut lengths = Vec::new();
    let mut values = Vec::new();
    for value in nested_values {
        lengths.push(value.len() as u64);
        values.extend(value);
    }
    println!("== pco + serde {type}");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let length_bytes = pco::standalone::simpler_compress(&lengths, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let value_bytes = pco::standalone::simpler_compress(&values, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let (length_bytes, value_bytes) = (serde_bytes::Bytes::new(&length_bytes), serde_bytes::Bytes::new(&value_bytes));
    let bytes = serde_json::to_vec(&(length_bytes, value_bytes)).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let (length_bytes, value_bytes) = serde_json::from_slice::<(Vec<u8>, Vec<u8>)>(&bytes).unwrap();
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

fn bench_flat_msgpack<T>(nested_values: Vec<Vec<T>>, r#type: &str)
where
    T: pco::data_types::Number,
{
    let mut lengths = Vec::new();
    let mut values = Vec::new();
    for value in nested_values {
        lengths.push(value.len() as u64);
        values.extend(value);
    }
    println!("== pco + msgpack {type}");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let length_bytes = pco::standalone::simpler_compress(&lengths, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let value_bytes = pco::standalone::simpler_compress(&values, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let (length_bytes, value_bytes) = (serde_bytes::Bytes::new(&length_bytes), serde_bytes::Bytes::new(&value_bytes));
    let bytes = rmp_serde::to_vec(&(length_bytes, value_bytes)).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let (length_bytes, value_bytes) = rmp_serde::from_slice::<(Vec<u8>, Vec<u8>)>(&bytes).unwrap();
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

fn bench_flat_cbor<T>(nested_values: Vec<Vec<T>>, r#type: &str)
where
    T: pco::data_types::Number,
{
    let mut lengths = Vec::new();
    let mut values = Vec::new();
    for value in nested_values {
        lengths.push(value.len() as u64);
        values.extend(value);
    }
    println!("== pco + cbor {type}");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let length_bytes = pco::standalone::simpler_compress(&lengths, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let value_bytes = pco::standalone::simpler_compress(&values, pco::DEFAULT_COMPRESSION_LEVEL).unwrap();
    let (length_bytes, value_bytes) = (serde_bytes::Bytes::new(&length_bytes), serde_bytes::Bytes::new(&value_bytes));
    let mut bytes = Vec::new();
    ciborium::into_writer(&(length_bytes, value_bytes), &mut bytes).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let (length_bytes, value_bytes): (Vec<u8>, Vec<u8>) = ciborium::from_reader(&bytes[..]).unwrap();
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

fn bench_msgpack<T>(values: Vec<T>, r#type: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    println!("== msgpack Vec<{type}>");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let bytes = msgpack_compress(values).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    for v in msgpack_decompress::<T>(&bytes) {
        black_box(v.unwrap());
    }
    println!("deserialized and discarded after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let result: Vec<T> = msgpack_decompress::<T>(&bytes).collect::<Result<_>>().unwrap();
    black_box(result);
    println!("deserialized and retained after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
}

fn bench_cbor<T>(values: Vec<T>, r#type: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    println!("== cbor Vec<{type}>");
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let bytes = cbor_compress(values).unwrap();
    println!("serialized to {:.0?} bytes after {:.1?} using {:.0?}KB peak memory", bytes.len(), start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    for v in cbor_decompress::<T>(&bytes) {
        black_box(v.unwrap());
    }
    println!("deserialized and discarded after {:.1?} using {:.0?}KB peak memory", start.elapsed(), PEAK_ALLOC.peak_usage_as_kb());
    PEAK_ALLOC.reset_peak_usage();
    let start = Instant::now();
    let result: Vec<T> = cbor_decompress::<T>(&bytes).collect::<Result<_>>().unwrap();
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

#[derive(serde::Serialize, serde::Deserialize)]
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

fn msgpack_compress<T>(items: Vec<T>) -> anyhow::Result<Vec<u8>>
where
    T: serde::Serialize,
{
    let mut output = Vec::new();
    let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
    for item in items {
        // Serialize directly into the zstd encoder
        rmp_serde::encode::write(&mut encoder, &item)?;
    }
    encoder.finish()?;
    Ok(output)
}

fn msgpack_decompress<T>(input: &[u8]) -> impl Iterator<Item = anyhow::Result<T>> + '_
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
        Err(e) => Some(Err(anyhow::anyhow!(e))),
    }))
}

fn cbor_compress<T>(items: Vec<T>) -> anyhow::Result<Vec<u8>>
where
    T: serde::Serialize,
{
    let mut output = Vec::new();
    let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
    for item in items {
        ciborium::into_writer(&item, &mut encoder)?;
    }
    encoder.finish()?;
    Ok(output)
}

fn cbor_decompress<T>(input: &[u8]) -> impl Iterator<Item = anyhow::Result<T>> + '_
where
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    let decoder = match zstd::stream::read::Decoder::new(input) {
        Ok(d) => d,
        Err(e) => return Box::new(std::iter::once(Err(e.into()))) as Box<dyn Iterator<Item = _>>,
    };
    let mut buffered = std::io::BufReader::with_capacity(128 * 1024, decoder);
    Box::new(std::iter::from_fn(move || match ciborium::from_reader::<T, _>(&mut buffered) {
        Ok(item) => Some(Ok(item)),
        Err(ciborium::de::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
        Err(e) => Some(Err(anyhow::anyhow!(e))),
    }))
}
