//! The Postgres adapter, compiled only under the `postgres`
//! feature so local-only builds never carry the Postgres stack.
//!
//! The `#[derive(PcoStore)]` derive writes the `Backend` impl per model: COPY,
//! SELECT, DELETE, and the generated typed WHERE over `index` columns and
//! timestamp bounds all need the model's field names, which only expansion
//! can see. This module holds the half that is model-agnostic: the pool.
//! Generated code reaches the Postgres types through `pco_store::tokio_postgres`,
//! so only this crate names the stack.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_stream::try_stream;
use futures::TryStreamExt;
use pco_pack::PcoPack;

use crate::backend::{BoxStream, Chunk};

/// The Postgres backend for any model: clients checked out per call from a
/// pooled connection, BYTEA payload columns stored exactly as the codec wrote
/// them while index columns stay plain so SQL keeps its teeth.
pub struct Postgres {
    pool: Arc<deadpool_postgres::Pool>,
}

impl Postgres {
    /// Build the adapter over `pool`.
    pub fn new(pool: Arc<deadpool_postgres::Pool>) -> Self {
        Self { pool }
    }

    /// Check out a client for one call. Multi-call transactions live above,
    /// so pco_store clients are borrowed per operation.
    pub async fn client(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.context("checkout postgres client")
    }
}

/// Where a streamed query gets its connection: the pool handle checks a
/// client out itself; a checked-out client is already one, which keeps the
/// `prepare_cached` statement cache on both paths.
pub enum Db<'a> {
    /// The pool: a client is checked out inside the stream, at first poll.
    Pool(&'a Postgres),
    /// A client the caller checked out: borrowed for the life of the stream.
    Borrowed(&'a deadpool_postgres::Object),
}

/// One load SELECT driven as a stream: prepare, `query_raw`, then hand each
/// row's chunk to the consumer as the server delivers it. The index narrowing
/// and timestamp bounds ride in the WHERE, so Postgres prunes partitions and
/// filters rows server-side; the per-request shape the no-query stores need
/// never costs a round trip here, because the single query itself pipelines.
pub fn query_stream<'a, M>(
    db: Db<'a>, sql: String, params: Vec<&'a (dyn tokio_postgres::types::ToSql + Sync)>,
    to_chunk: impl Fn(&tokio_postgres::Row) -> Chunk<M> + Send + 'a,
) -> BoxStream<'a, Result<Chunk<M>>>
where
    M: PcoPack + 'a,
    M::Chunk: Send,
    M::Filter: Send,
{
    let stream = try_stream! {
        let guard;
        let db: &deadpool_postgres::Object = match db {
            Db::Pool(pool) => {
                guard = pool.client().await?;
                &guard
            }
            Db::Borrowed(client) => client,
        };
        let stmt = db.prepare_cached(&sql).await?;
        // The RowStream borrows the connection and is !Unpin, so it is pinned before polling.
        let rows = db.query_raw(&stmt, params).await?;
        let mut rows = std::pin::pin!(rows);
        while let Some(row) = rows.try_next().await? {
            yield to_chunk(&row);
        }
    };
    Box::pin(stream)
}
