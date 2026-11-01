use chrono::TimeDelta;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{ServiceType, Slot};

pub async fn free_technicians(
    conn: &mut PgConnection,
    dealership_id: Uuid,
    required_skill: &str,
    slot: Slot,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT t.id
           FROM technician t
           WHERE t.dealership_id = $1
             AND $2 = ANY (t.skills)
             AND NOT EXISTS (
                 SELECT 1 FROM appointment a
                 WHERE a.technician_id = t.id
                   AND a.status = 'confirmed'
                   AND a.slot && tstzrange($3, $4, '[)'))
           ORDER BY t.id"#,
        dealership_id,
        required_skill,
        slot.start,
        slot.end,
    )
    .fetch_all(conn)
    .await
}

pub async fn free_bays(
    conn: &mut PgConnection,
    dealership_id: Uuid,
    slot: Slot,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT b.id
           FROM service_bay b
           WHERE b.dealership_id = $1
             AND NOT EXISTS (
                 SELECT 1 FROM appointment a
                 WHERE a.bay_id = b.id
                   AND a.status = 'confirmed'
                   AND a.slot && tstzrange($2, $3, '[)'))
           ORDER BY b.id"#,
        dealership_id,
        slot.start,
        slot.end,
    )
    .fetch_all(conn)
    .await
}

pub async fn booking_refs_exist(
    conn: &mut PgConnection,
    customer_id: Uuid,
    vehicle_id: Uuid,
    dealership_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1
               FROM vehicle v
               JOIN customer c ON c.id = v.customer_id
               JOIN dealership d ON d.id = $3
               WHERE v.id = $2 AND c.id = $1
           ) AS "exists!""#,
        customer_id,
        vehicle_id,
        dealership_id,
    )
    .fetch_one(conn)
    .await
}

pub async fn find_service_type(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<ServiceType>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT id, duration_minutes, required_skill FROM service_type WHERE id = $1",
        id,
    )
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|r| ServiceType {
        id: r.id,
        duration: TimeDelta::minutes(i64::from(r.duration_minutes)),
        required_skill: r.required_skill,
    }))
}

pub async fn dealership_exists(conn: &mut PgConnection, id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM dealership WHERE id = $1) AS "exists!""#,
        id,
    )
    .fetch_one(conn)
    .await
}
