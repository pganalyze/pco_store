#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

#[cfg(feature = "postgres")]
pub mod pg;

pub mod backend;
pub mod fs;

pub use anyhow;
pub use backend::{Backend, BoxFuture, Capabilities, Chunk, Filter, PcoStore, decode_chunk, encode_chunk};
pub use pco_store_derive::{PcoStore, pco_store};

// Re-exports for the code `#[derive(PcoStore)]` generates: generated
// adapters name the packing layer and the Postgres stack through
// `pco_store`, so user crates depend on pco_store alone and never pull
// pco_pack into scope themselves.
pub use pco_pack;
#[cfg(feature = "postgres")]
pub use {deadpool_postgres, tokio_postgres};

/// Re-export of pco_pack's `fold!` macro, the fast path for `Chunk::for_each`
/// and `PcoStore::for_each`: its closure decodes only the columns it reads.
pub use pco_pack::fold;
