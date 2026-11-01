use std::ops::Range;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub async fn open_starts(
    conn: &mut PgConnection,
    dealership_id: Uuid,
    skill: &str,
    day: Range<DateTime<Utc>>,
    step: TimeDelta,
    duration: TimeDelta,
    now: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT s AS "start!"
           FROM generate_series($2::timestamptz, $3::timestamptz, $4::bigint * interval '1 second') AS s
           WHERE s < $3 AND s >= $5
             AND EXISTS (
                 SELECT 1 FROM service_bay b
                 WHERE b.dealership_id = $1
                   AND NOT EXISTS (
                       SELECT 1 FROM appointment a
                       WHERE a.bay_id = b.id
                         AND a.status = 'confirmed'
                         AND a.slot && tstzrange(s, s + $6::bigint * interval '1 second', '[)')))
             AND EXISTS (
                 SELECT 1 FROM technician t
                 WHERE t.dealership_id = $1
                   AND $7 = ANY (t.skills)
                   AND NOT EXISTS (
                       SELECT 1 FROM appointment a
                       WHERE a.technician_id = t.id
                         AND a.status = 'confirmed'
                         AND a.slot && tstzrange(s, s + $6::bigint * interval '1 second', '[)')))
           ORDER BY s"#,
        dealership_id,
        day.start,
        day.end,
        step.num_seconds(),
        now,
        duration.num_seconds(),
        skill,
    )
    .fetch_all(conn)
    .await
}
