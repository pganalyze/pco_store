//! The storage seam: one backend trait, one `impl` per adapter,
//! selected by crate features: `#[derive(PcoStore)]` emits one impl per
//! model per enabled backend. The trait lives in the runtime so a store can
//! hold one runtime-picked
//! `dyn Backend<M>` and switch adapters across ladder rungs without any call
//! site changing. The `PcoStore` trait on the other side of the seam
//! answers `store`, `load`, `delete`, and `for_each` at the model name,
//! delegating here.
//!
//! The macro that emits these impls reads the model's field names (a runtime
//! generic could not build a WHERE clause, a file name, or an object key
//! without seeing them), so most of what the derive generates is
//! backend-specific surface. The whole-chunk codec below is model-agnostic
//! and lives here once.

use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;

use anyhow::Result;
use futures::TryStreamExt;
use pco_pack::PcoPack;

/// A future returned by a backend method. Boxed so `Backend` stays object
/// safe, which is what lets a store hold a runtime-picked `dyn Backend<M>`
///. Callsites that want no boxing take `impl Backend<M>` generics.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stream returned by a backend method. Boxed for the same reason as
/// [`BoxFuture`]. Consuming it needs only the `TryStreamExt` re-export at the
/// crate root (`try_next` to pull chunk by chunk, `try_collect` for a `Vec`).
pub type BoxStream<'a, T> = Pin<Box<dyn futures::stream::Stream<Item = T> + Send + 'a>>;

/// What an adapter honestly promises. Callers consult this and
/// never learn a cloud's atomicity by reading Postgres's, because the trait
/// will not flatten a cloud down to Postgres's guarantees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// The adapter can commit a multi-write unit atomically.
    pub transactions: bool,
    /// The adapter can rename one persisted object into place atomically.
    pub rename: bool,
    /// The adapter can read a byte range of a stored object.
    pub range_get: bool,
    /// The adapter can push a filter down into storage before reading bytes.
    pub sql_pushdown: bool,
}

/// A chunk as the storage layer holds it: pco_pack's serialization-level
/// chunk, the filter it was read under, and the fields it was requested
/// with. Chunks from `delete` carry an empty filter, so decompressing them
/// yields every row in the chunk and re-storing a compaction never loses
/// rows outside the queried range. The wrapper derefs to the pco_pack chunk
/// so index columns and the `start_at`/`end_at` bounds read directly.
pub struct Chunk<M: PcoPack> {
    /// The pco_pack chunk as stored; deref reaches its fields.
    pub chunk: M::Chunk,
    /// The filter applied at read time; empty for chunks from `delete`.
    pub filter: M::Filter,
    /// The columns to keep when decompressing; set by `load`, empty by `delete`.
    pub fields: Vec<String>,
}

impl<M: PcoPack> Deref for Chunk<M> {
    type Target = M::Chunk;

    fn deref(&self) -> &M::Chunk {
        &self.chunk
    }
}

impl<M: PcoPack> Chunk<M>
where
    M::Filter: Clone,
    <M::Filter as TryInto<pco_pack::serde_json::Value>>::Error: Into<anyhow::Error>,
{
    /// Decompresses this chunk under the filter and fields it was loaded with.
    /// Chunks from `delete` keep an empty filter, so `decompress` answers every
    /// row they hold and a re-store loses nothing.
    pub fn decompress(&self) -> Result<Vec<M>> {
        let fields: Vec<&str> = self.fields.iter().map(String::as_str).collect();
        M::filter(std::slice::from_ref(&self.chunk), self.filter.clone(), &fields)
    }

    /// Streams this chunk's rows matching the stored filter into `consumer`
    /// without materializing row structs. A `fold!`-style consumer decodes
    /// only the columns its closure reads, which is the fast path for
    /// aggregates over big chunks.
    pub fn for_each<C>(&self, consumer: C) -> Result<()>
    where
        C: pco_pack::Consumer<M>,
    {
        let fields: Vec<&str> = self.fields.iter().map(String::as_str).collect();
        M::for_each(std::slice::from_ref(&self.chunk), self.filter.clone(), &fields, consumer)
    }
}

/// The typed filter pco_pack generated for model `M`, re-exported so call
/// sites reach it without importing `PcoPack`: alias it (`type Filter =
/// pco_store::Filter<FsStat>;`) and construct as usual, or write
/// `pco_store::Filter::<FsStat>::new(..)` inline. A crate-level alias rather
/// than `Model::Filter`: associated types resolve through every trait a type
/// implements, import or not, so a name `Filter` on the model would sit
/// ambiguously beside `PcoPack::Filter`.
pub type Filter<M> = <M as pco_pack::PcoPack>::Filter;

/// One storage adapter for one model. `store` persists whole
/// chunks; `load` narrows by the model's `index` fields and timestamp bounds
/// and streams the survivors with the filter and fields applied; `delete`
/// returns the whole chunks it removed for lossless compaction.
/// The shared lifetime `'a` spans the adapter and its arguments, which is
/// what the boxed futures and streams borrow while they run.
pub trait Backend<M: PcoPack>: Send + Sync {
    /// Persist these chunks whole.
    fn store<'a>(&'a self, chunks: &'a [M::Chunk]) -> BoxFuture<'a, Result<()>>;

    /// Stream the chunks whose key and bounds match `filter`, keeping `fields`.
    /// Each adapter chooses its own request shape behind this one signature:
    /// the filesystem adapter lists names once and answers each chunk as its
    /// file is read; Postgres runs one narrowed SELECT through `query_raw`,
    /// so rows stream as the server produces them; a future object adapter
    /// lists one key prefix per index tuple and pipelines the resulting
    /// point GETs. Stopping the stream early stops paying for it.
    fn load<'a>(&'a self, filter: &'a M::Filter, fields: &'a [&str]) -> BoxStream<'a, Result<Chunk<M>>>;

    /// Delete the chunks matching `filter`, returning each whole chunk with an
    /// empty filter so a re-store of its rows loses nothing.
    fn delete<'a>(&'a self, filter: &'a M::Filter) -> BoxFuture<'a, Result<Vec<Chunk<M>>>>;

    /// The atomicity this adapter can honestly offer.
    fn capabilities(&self) -> Capabilities;
}

/// The model's face of the storage calls. `#[derive(PcoStore)]` emits an
/// empty impl of this trait, and the default methods here do all the work,
/// so `QueryStat::store(db, rows)` reads like data access through any
/// `&Backend` while the model's inherent surface stays clean. The trait
/// shares the derive's name the way serde does: one `use pco_store::PcoStore`
/// brings both into scope.
pub trait PcoStore: PcoPack {
    /// Persists rows, grouped into chunks first, through any backend.
    fn store<'a, B>(backend: &'a B, rows: Vec<Self>) -> BoxFuture<'a, Result<()>>
    where
        Self: 'a + Send,
        B: Backend<Self> + ?Sized,
        Self::Chunk: Send,
    {
        Box::pin(async move {
            let chunks = <Self as PcoPack>::write(rows)?;
            backend.store(&chunks).await
        })
    }

    /// Streams the chunks matching `filter`, keeping `fields`, through any
    /// backend.
    fn load<'a, B>(backend: &'a B, filter: &'a Self::Filter, fields: &'a [&str]) -> BoxStream<'a, Result<Chunk<Self>>>
    where
        B: Backend<Self> + ?Sized,
    {
        backend.load(filter, fields)
    }

    /// Deletes the chunks matching `filter` through any backend, returning
    /// each whole chunk with an empty filter so a re-store loses nothing.
    fn delete<'a, B>(backend: &'a B, filter: &'a Self::Filter) -> BoxFuture<'a, Result<Vec<Chunk<Self>>>>
    where
        B: Backend<Self> + ?Sized,
    {
        backend.delete(filter)
    }

    /// Streams the chunks matching `filter` and folds each one through
    /// `consumer` as it loads, without materializing row structs or a chunk
    /// vector: the first fold begins at the first chunk. One consumer spans
    /// the whole load, so a `fold!` accumulator sums across chunks and decodes
    /// only the columns it reads. Feeding pco_pack's public per-chunk fold
    /// engine (`fold_value`) chunk by chunk is exactly what one call over a
    /// whole slice expands to: `Consumer` cursors are forward-only per chunk
    /// and `fold!` keeps its state in the consumer, so no fold state lives
    /// between chunks. Named to match `PcoPack::for_each`: pco_store wraps
    /// pco_pack (see the crate docs), so `PcoPack` stays out of scope and the
    /// model path stays unambiguous.
    fn for_each<'a, B, C>(backend: &'a B, filter: &'a Self::Filter, fields: &'a [&str], consumer: C) -> BoxFuture<'a, Result<()>>
    where
        Self: 'a,
        B: Backend<Self> + ?Sized,
        C: pco_pack::Consumer<Self> + Send + 'a,
        Self::Filter: Clone + Sync,
        <Self::Filter as TryInto<pco_pack::serde_json::Value>>::Error: Into<anyhow::Error>,
    {
        Box::pin(async move {
            let filter_json: pco_pack::serde_json::Value = filter.clone().try_into().map_err(Into::into)?;
            let mut consumer = consumer;
            let mut stream = backend.load(filter, fields);
            while let Some(chunk) = stream.try_next().await? {
                <Self as PcoPack>::fold_value(std::slice::from_ref(&chunk.chunk), &filter_json, fields, &mut consumer)?;
            }
            Ok(())
        })
    }
}

/// Encode one chunk for the byte-native adapters by delegating to pco_pack's
/// chunk container (`to_bytes`), so persisted bytes match pco_pack's
/// serialized chunks field for field and follow any layout change upstream.
/// The slice of one is deliberate: one file or object stays one
/// independently addressable, independently renameable chunk.
pub fn encode_chunk<M: PcoPack>(chunk: &M::Chunk) -> Result<Vec<u8>> {
    M::to_bytes(std::slice::from_ref(chunk))
}

/// Decode one chunk read whole, via pco_pack's `from_bytes`. Adapters persist
/// files through `encode_chunk`, so a container that does not hold exactly
/// one chunk is a foreign file, and it is named as such rather than
/// half-decoded.
pub fn decode_chunk<M: PcoPack>(bytes: &[u8]) -> Result<M::Chunk> {
    let mut chunks = M::from_bytes(bytes)?;
    anyhow::ensure!(chunks.len() == 1, "expected one chunk in these bytes, found {}", chunks.len());
    Ok(chunks.pop().expect("length checked"))
}

/// A stream answering with one error. A stream-returning method that fails
/// while planning (a filter that will not convert, a field that will not
/// resolve) reports it as the stream's only item instead of a signature that
/// is both `Result` and `Stream`.
pub fn stream_err<M: PcoPack>(err: anyhow::Error) -> BoxStream<'static, Result<Chunk<M>>> {
    Box::pin(futures::stream::once(async move { Err::<Chunk<M>, _>(err) }))
}
