#![cfg(feature = "fs")]
//! The filesystem adapter end to end: real files on real disks,
//! chunks addressed by name.

use std::fs;

use chrono::{DateTime, TimeZone, Utc};
use pco_store::fs::Fs;
use pco_store::{Backend, PcoStore, pco_store};

#[derive(Clone, Debug, PartialEq, PcoStore)]
#[pco_store(timestamp = collected_at, index = [database_id])]
pub struct FsStat {
    pub database_id: i64,
    pub collected_at: DateTime<Utc>,
    pub calls: i64,
}

/// pco_store re-exports pco_pack's typed filter as `pco_store::Filter<M>`,
/// so the wrapped API needs no pco_pack import.
type Filter = pco_store::Filter<FsStat>;

fn rows(chunks: &[pco_store::Chunk<FsStat>]) -> Vec<FsStat> {
    let mut rows = Vec::new();
    for chunk in chunks {
        rows.extend(chunk.decompress().unwrap());
    }
    rows
}

fn file_names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn fs_store_load_narrow_and_delete() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = Fs::open(dir.path()).unwrap();
    let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    // One chunk per store call, one file per chunk.
    FsStat::store(
        &store,
        vec![
            FsStat { database_id: 1, collected_at: base + chrono::Duration::minutes(1), calls: 1 },
            FsStat { database_id: 1, collected_at: base + chrono::Duration::minutes(2), calls: 2 },
        ],
    )
    .await
    .unwrap();
    FsStat::store(&store, vec![FsStat { database_id: 2, collected_at: base + chrono::Duration::minutes(1), calls: 5 }]).await.unwrap();

    let files = file_names(dir.path());
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|name| name.starts_with("fs_stats~")), "{files:?}");
    // The file on disk is a pco_pack chunk container: decode_chunk reads it
    // back whole, outside any backend call.
    let raw = fs::read(dir.path().join(&files[0])).unwrap();
    assert_eq!(pco_store::decode_chunk::<FsStat>(&raw).unwrap().database_id, 1);

    // Narrowing by key answers at readdir: only database_id 1's file is read.
    let window = (base - chrono::Duration::hours(1))..=(base + chrono::Duration::hours(1));
    let chunks = FsStat::load(&store, &Filter::new([1i64], window.clone()), &[]).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(rows(&chunks).iter().map(|r| r.calls).sum::<i64>(), 3);

    // The full window reads both files.
    let all = FsStat::load(&store, &Filter::new([1i64, 2i64], window.clone()), &[]).await.unwrap();
    assert_eq!(rows(&all).iter().map(|r| r.calls).sum::<i64>(), 8);

    // Re-storing identical chunk bytes overwrites its own name: no duplicates.
    let first_file = dir.path().join(file_names(dir.path()).first().unwrap().clone());
    let _ = first_file;
    FsStat::store(
        &store,
        vec![
            FsStat { database_id: 1, collected_at: base + chrono::Duration::minutes(1), calls: 1 },
            FsStat { database_id: 1, collected_at: base + chrono::Duration::minutes(2), calls: 2 },
        ],
    )
    .await
    .unwrap();
    assert_eq!(file_names(dir.path()).len(), 2);

    // Delete answers with whole chunks and empty filters, so a re-store
    // loses nothing, and its files are gone.
    let gone = FsStat::delete(&store, &Filter::new([2i64], window.clone())).await.unwrap();
    assert_eq!(gone.len(), 1);
    assert_eq!(rows(&gone).iter().map(|r| r.calls).sum::<i64>(), 5);
    assert_eq!(file_names(dir.path()).len(), 1);

    // With delete, the surviving file is database_id 1's chunk, intact.
    let survivors = FsStat::load(&store, &Filter::new([1i64], window), &[]).await.unwrap();
    assert_eq!(rows(&survivors).iter().map(|r| r.calls).sum::<i64>(), 3);

    assert!(store.capabilities().rename);
}

#[tokio::test]
async fn chunks_deref_and_for_each() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = Fs::open(dir.path()).unwrap();
    let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let window = base..=base + chrono::Duration::hours(1);

    FsStat::store(
        &store,
        vec![
            FsStat { database_id: 1, collected_at: base, calls: 2 },
            FsStat { database_id: 1, collected_at: base + chrono::Duration::minutes(1), calls: 3 },
        ],
    )
    .await
    .unwrap();
    FsStat::store(&store, vec![FsStat { database_id: 2, collected_at: base, calls: 7 }]).await.unwrap();

    let chunks = FsStat::load(&store, &Filter::new([1i64], window.clone()), &[]).await.unwrap();
    assert_eq!(chunks.len(), 1);

    // Deref reaches the stored chunk, so index columns and the timestamp
    // bounds read directly off the load result.
    assert_eq!(chunks[0].database_id, 1);
    assert!(chunks[0].start_at <= chunks[0].end_at);

    // A chunk folds itself through a consumer, no row vector materialized.
    let mut calls = 0;
    chunks[0].for_each(|row: FsStat| calls += row.calls).unwrap();
    assert_eq!(calls, 5);

    // The model static folds a whole load with one consumer.
    let mut both = 0;
    FsStat::for_each(&store, &Filter::new([1i64, 2i64], window), &[], |row: FsStat| both += row.calls).await.unwrap();
    assert_eq!(both, 12);
}
