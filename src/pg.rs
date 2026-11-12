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
