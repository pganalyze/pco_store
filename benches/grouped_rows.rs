#![allow(dead_code)]

use ahash::AHashMap;
use brunch::{Bench, Benches};
use chrono::{DateTime, DurationRound, Utc, TimeZone};
use std::collections::HashMap;

fn main() {
    let mut benches = Benches::default();

    let mut dates: Vec<DateTime<Utc>> = vec![];
    for m in 1..13 {
        for d in 1..29 {
            let time: DateTime<Utc> = Utc.with_ymd_and_hms(2025, m, d, 13, 45, 43).single().unwrap();
            let date = time.duration_trunc(chrono::Duration::days(1)).unwrap();
            dates.push(date);
        }
    }

    let mut rows: Vec<Option<usize>> = vec![];
    for i in 0..5000 {
        rows.push(Some(i));
    }

    benches.push(
        Bench::new("HashMap")
        .run(|| {
            let mut grouped_rows: HashMap<_, Vec<_>> = HashMap::new();
            for date in &dates {
                for row in &rows {
                    grouped_rows.entry(date).or_default().push(row);
                }
            }
        })
    );

    benches.push(
        Bench::new("AHashMap")
        .run(|| {
            let mut grouped_rows: AHashMap<_, Vec<_>> = AHashMap::new();
            for date in &dates {
                for row in &rows {
                    grouped_rows.entry(date).or_default().push(row);
                }
            }
        })
    );

    benches.finish();
}
