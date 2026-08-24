use chrono::{DateTime, NaiveDate, Utc};

fn ymd_hms_micros(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, micros: u32) -> Option<DateTime<Utc>> {
    Some(NaiveDate::from_ymd_opt(year, month, day)?.and_hms_micro_opt(hour, min, sec, micros)?.and_utc())
}

// SystemTime support has been removed in favor of chrono::DateTime<Utc> only.
// This test verifies that the new API works correctly with chrono timestamps.
#[tokio::test]
#[serial_test::serial]
async fn chrono_timestamp() {
    #[derive(Clone, Eq, Hash, PartialEq)]
    #[pco_store::store(timestamp = collected_at, index = [database_id])]
    pub struct QueryStat {
        pub database_id: i64,
        pub collected_at: chrono::DateTime<Utc>,
    }

    let database_id = 1;
    let start = ymd_hms_micros(2026, 01, 01, 1, 01, 02, 345_678).unwrap();
    let end = ymd_hms_micros(2026, 01, 01, 5, 03, 04, 567_890).unwrap();

    let db = &super::DB_POOL.get().await.unwrap();
    let sql = "
        DROP TABLE IF EXISTS query_stats;
        CREATE TABLE query_stats (
            database_id bigint NOT NULL,
            start_at timestamptz NOT NULL,
            end_at timestamptz NOT NULL,
            collected_at bytea STORAGE EXTERNAL NOT NULL
        );
        CREATE INDEX ON query_stats USING btree (database_id);
        CREATE INDEX ON query_stats USING btree (end_at, start_at);
    ";
    db.batch_execute(sql).await.unwrap();

    let time_1_23_45 = ymd_hms_micros(2026, 01, 01, 1, 23, 45, 123_456).unwrap();
    let time_2_34_56 = ymd_hms_micros(2026, 01, 01, 2, 34, 56, 789_012).unwrap();
    let time_3_45_00 = ymd_hms_micros(2026, 01, 01, 3, 45, 00, 345_678).unwrap();

    // Write first two rows
    let stats = vec![QueryStat { database_id, collected_at: time_1_23_45 }];
    CompressedQueryStats::store(db, stats).await.unwrap();
    let stats = vec![QueryStat { database_id, collected_at: time_2_34_56 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read back all rows in range
    {
        let mut actual = Vec::new();
        for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
            actual.extend(chunk.decompress().unwrap());
        }
        assert_eq!(actual.len(), 2);
        assert_eq!(actual[0].collected_at, time_1_23_45);
        assert_eq!(actual[1].collected_at, time_2_34_56);
    }

    // Write a third row
    let stats = vec![QueryStat { database_id, collected_at: time_3_45_00 }];
    CompressedQueryStats::store(db, stats).await.unwrap();

    // Read back all three rows
    {
        let mut actual = Vec::new();
        for chunk in CompressedQueryStats::load(db, Filter::new([database_id], start..=end), &[]).await.unwrap() {
            actual.extend(chunk.decompress().unwrap());
        }
        assert_eq!(actual.len(), 3);
        assert_eq!(actual[0].collected_at, time_1_23_45);
        assert_eq!(actual[1].collected_at, time_2_34_56);
        assert_eq!(actual[2].collected_at, time_3_45_00);
    }
}
