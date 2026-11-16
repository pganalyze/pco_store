//! The storage seam: one backend trait, one `impl` per adapter,
//! selected by crate features: `#[derive(PcoStore)]` emits one impl per
//! model per enabled backend. The trait lives in the runtime so a store can
//! hold one runtime-picked
//! `dyn Backend<M>` and switch adapters across ladder rungs without any call
//! site changing. The `PcoStore` trait on the other side of the seam
//! answers `store`, `load`, `delete`, `for_each`, and `for_each_parallel` at
//! the model name, delegating here.
//!
//! The macro that emits these impls reads the model's field names (a runtime
//! generic could not build a WHERE clause, a file name, or an object key
//! without seeing them), so most of what the derive generates is
//! backend-specific surface. The whole-chunk codec below is model-agnostic
//! and lives here once.

use std::collections::BTreeMap;
use std::future::Future;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use futures::StreamExt;
use futures::TryStreamExt;
use futures::channel::mpsc;
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

    /// Folds a whole load through `consumer` exactly as `for_each` does —
    /// same rows, same order, one serial consumer whose captured state stays
    /// the idiomatic place for accumulators — while chunk decode runs ahead on
    /// worker threads: up to `workers` decoded chunks wait in the pipeline
    /// while the main task folds the previous one, so a decode-bound
    /// projection pays roughly one decode plus one fold walk instead of the
    /// two interleaved. Best-effort: a fold-heavy consumer (a `HashMap` insert
    /// can outlast the decode of its own chunk) sees little, and a load of one
    /// chunk gains nothing, so stay on `for_each` there. Abandoning the fold
    /// early wastes the at-most-`workers` preparations the detached threads are
    /// already running plus the queued chunks behind them: preparation threads
    /// cannot be cancelled once started, and chunks past a failing fold keep
    /// loading until the drop notices. A panic inside a preparation thread
    /// surfaces here as an error rather than aborting the fold task. Models
    /// whose decoded chunk is not `Send` fail to compile only at this call
    /// site.
    fn for_each_parallel<'a, B, C>(
        backend: &'a B, filter: &'a Self::Filter, fields: &'a [&str], consumer: C, workers: NonZeroUsize,
    ) -> BoxFuture<'a, Result<()>>
    where
        Self: 'a,
        B: Backend<Self> + ?Sized,
        C: pco_pack::Consumer<Self> + Send + 'a,
        Self::Filter: Clone + Sync,
        Self::Chunk: Send + 'static,
        Self::Prepared: Send + 'static,
        <Self::Filter as TryInto<pco_pack::serde_json::Value>>::Error: Into<anyhow::Error>,
    {
        Box::pin(async move {
            let filter_json: pco_pack::serde_json::Value = filter.clone().try_into().map_err(Into::into)?;
            let mut consumer = consumer;
            let mut load = backend.load(filter, fields);
            // The window is `workers` chunks plus the one in flight through
            // the fold; every issued-but-unfolded chunk (queued, decoding, or
            // decoded in `reorder`) counts against it, so peak decoded memory
            // stays near `workers + 1` chunks.
            let window = workers.get() as u64 + 1;
            let (job_tx, job_rx) = std_mpsc::sync_channel::<(u64, Self::Chunk)>(window as usize);
            let job_rx = Arc::new(Mutex::new(job_rx));
            // Unbounded by declaration yet bounded in fact: every chunk
            // answers exactly one result and the window keeps
            // issued-but-unfolded chunks at `workers + 1`, so the queue never
            // holds more decoded chunks than that. An unbounded queue also
            // lets a thread hand off a result without a blocking call, which
            // the futures channel API does not offer.
            let (done_tx, mut done_rx) = mpsc::unbounded::<(u64, Result<Self::Prepared>)>();
            // The plan and projection, and the threads that consume them,
            // materialize with the first chunk only: an empty load must answer
            // Ok without validating, exactly as `for_each` does (whose
            // per-chunk `fold_value` never runs, so never rejects a bad
            // projection and never fires an enum's explicit-fields check, over
            // zero chunks).
            let mut plan: Option<Arc<(Vec<pco_pack::ResolvedFilter>, Vec<&'static str>)>> = None;
            let mut issued: u64 = 0;
            let mut yielded: u64 = 0;
            let mut reorder: BTreeMap<u64, Result<Self::Prepared>> = BTreeMap::new();
            let mut load_done = false;
            loop {
                // Threads finish chunks out of order; the fold walks strictly
                // in load order, so `for_each` and `for_each_parallel` deliver
                // identical rows and order-sensitive folds agree.
                while let Some(head) = reorder.remove(&yielded) {
                    let mut prepared = head?;
                    <Self as PcoPack>::fold_prepared(&mut prepared, &mut consumer)?;
                    yielded += 1;
                }
                if load_done && issued == yielded {
                    return Ok(());
                }
                while !load_done && issued - yielded < window {
                    match load.try_next().await? {
                        // Stripping the wrapper here, as a match temporary
                        // dropped before the next await, keeps its non-`Send`
                        // stored filter out of the future; `for_each` ignores
                        // those stored two fields, so we do too.
                        Some(Chunk { chunk, .. }) => {
                            if plan.is_none() {
                                let mut base = fields.to_vec();
                                base.extend_from_slice(<C as pco_pack::Consumer<Self>>::FIELDS);
                                let p = Arc::new((
                                    <Self as pco_pack::PcoFilter>::resolve_query(&filter_json)?,
                                    <Self as PcoPack>::resolve_fields(&filter_json, &base)?,
                                ));
                                for _ in 0..workers.get() {
                                    let job_rx = Arc::clone(&job_rx);
                                    let plan = Arc::clone(&p);
                                    let send = done_tx.clone();
                                    let exit_tx = done_tx.clone();
                                    std::thread::spawn(move || {
                                        // The guard lives inside the thread on
                                        // purpose: it marks this thread's exit,
                                        // and a guard left in the spawning
                                        // scope would fire its sentinel the
                                        // moment spawn returned.
                                        let _exit = WorkerExit(Some(exit_tx));
                                        loop {
                                            let (seq, chunk) = match job_rx.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).recv() {
                                                Ok(job) => job,
                                                Err(std_mpsc::RecvError) => break,
                                            };
                                            let prepared = <Self as PcoPack>::prepare(&chunk, &plan.0, &plan.1);
                                            if send.unbounded_send((seq, prepared)).is_err() {
                                                break;
                                            }
                                        }
                                    });
                                }
                                plan = Some(p);
                            }
                            match job_tx.try_send((issued, chunk)) {
                                Ok(()) => issued += 1,
                                // Unreachable: the window check bounds queued jobs below the queue capacity.
                                Err(std_mpsc::TrySendError::Full(_)) => return Err(anyhow!("chunk preparation window overran its job queue")),
                                Err(std_mpsc::TrySendError::Disconnected(_)) => {
                                    return Err(anyhow!("chunk preparation threads stopped before every chunk was prepared"));
                                }
                            }
                        }
                        None => load_done = true,
                    }
                }
                match done_rx.next().await {
                    Some((seq, prepared)) if seq == u64::MAX => {
                        return Err(prepared.err().unwrap_or_else(|| anyhow!("a chunk preparation thread exited unexpectedly")));
                    }
                    Some((seq, prepared)) => {
                        reorder.insert(seq, prepared);
                    }
                    None => return Err(anyhow!("chunk preparation threads stopped before every chunk was prepared")),
                }
            }
        })
    }
}

/// One preparation thread's exit notice: its `Drop`, whether the loop ended
/// or a panic unwound through it, sends the sentinel item. The fold turns a
/// sentinel into an error while it is still waiting on work, so a thread that
/// dies with the fold alive can never strand a chunk mid-pipeline. When the
/// fold is already gone the send fails silently.
struct WorkerExit<P>(Option<mpsc::UnboundedSender<(u64, Result<P>)>>);

impl<P> Drop for WorkerExit<P> {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.unbounded_send((u64::MAX, Err(anyhow!("a chunk preparation thread exited unexpectedly"))));
        }
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
