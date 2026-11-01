use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::{Appointment, AppointmentStatus, BookingRequest, Slot};

const UNIQUE_VIOLATION: &str = "23505";
const EXCLUSION_VIOLATION: &str = "23P01";

const BAY_OVERLAP: &str = "ex_appointment_bay_id_slot";
const TECHNICIAN_OVERLAP: &str = "ex_appointment_technician_id_slot";
const VEHICLE_OVERLAP: &str = "ex_appointment_vehicle_id_slot";
const IDEMPOTENCY_KEY_UNIQUE: &str = "uq_appointment_customer_id_idempotency_key";

pub enum InsertOutcome {
    Created(Appointment),
    BayTaken,
    TechnicianTaken,
    VehicleTaken,
    KeyExists,
}

struct AppointmentRow {
    id: Uuid,
    customer_id: Uuid,
    vehicle_id: Uuid,
    dealership_id: Uuid,
    service_type_id: Uuid,
    bay_id: Uuid,
    technician_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    status: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<AppointmentRow> for Appointment {
    type Error = sqlx::Error;

    fn try_from(row: AppointmentRow) -> Result<Self, Self::Error> {
        let status = AppointmentStatus::parse(&row.status)
            .ok_or_else(|| sqlx::Error::Decode("unknown appointment status".into()))?;
        Ok(Self {
            id: row.id,
            customer_id: row.customer_id,
            vehicle_id: row.vehicle_id,
            dealership_id: row.dealership_id,
            service_type_id: row.service_type_id,
            bay_id: row.bay_id,
            technician_id: row.technician_id,
            slot: Slot {
                start: row.start,
                end: row.end,
            },
            status,
            created_at: row.created_at,
        })
    }
}

fn classify(err: sqlx::Error) -> Result<InsertOutcome, sqlx::Error> {
    let Some(db_err) = err.as_database_error() else {
        return Err(err);
    };
    match (db_err.code().as_deref(), db_err.constraint()) {
        (Some(EXCLUSION_VIOLATION), Some(BAY_OVERLAP)) => Ok(InsertOutcome::BayTaken),
        (Some(EXCLUSION_VIOLATION), Some(TECHNICIAN_OVERLAP)) => Ok(InsertOutcome::TechnicianTaken),
        (Some(EXCLUSION_VIOLATION), Some(VEHICLE_OVERLAP)) => Ok(InsertOutcome::VehicleTaken),
        (Some(UNIQUE_VIOLATION), Some(IDEMPOTENCY_KEY_UNIQUE)) => Ok(InsertOutcome::KeyExists),
        _ => Err(err),
    }
}

pub async fn insert_appointment(
    conn: &mut PgConnection,
    req: &BookingRequest,
    slot: Slot,
    bay_id: Uuid,
    technician_id: Uuid,
) -> Result<InsertOutcome, sqlx::Error> {
    let inserted = sqlx::query_as!(
        AppointmentRow,
        r#"INSERT INTO appointment
               (customer_id, vehicle_id, dealership_id, service_type_id,
                bay_id, technician_id, slot, idempotency_key)
           VALUES ($1, $2, $3, $4, $5, $6, tstzrange($7, $8, '[)'), $9)
           RETURNING id, customer_id, vehicle_id, dealership_id, service_type_id,
                     bay_id, technician_id,
                     lower(slot) AS "start!", upper(slot) AS "end!",
                     status, created_at"#,
        req.customer_id,
        req.vehicle_id,
        req.dealership_id,
        req.service_type_id,
        bay_id,
        technician_id,
        slot.start,
        slot.end,
        req.idempotency_key,
    )
    .fetch_one(conn)
    .await;
    match inserted {
        Ok(row) => Ok(InsertOutcome::Created(row.try_into()?)),
        Err(err) => classify(err),
    }
}

pub async fn find_appointment_by_key(
    conn: &mut PgConnection,
    customer_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<Appointment>, sqlx::Error> {
    sqlx::query_as!(
        AppointmentRow,
        r#"SELECT id, customer_id, vehicle_id, dealership_id, service_type_id,
                  bay_id, technician_id,
                  lower(slot) AS "start!", upper(slot) AS "end!",
                  status, created_at
           FROM appointment
           WHERE customer_id = $1 AND idempotency_key = $2"#,
        customer_id,
        idempotency_key,
    )
    .fetch_optional(conn)
    .await?
    .map(Appointment::try_from)
    .transpose()
}

pub async fn find_appointment_by_id(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Appointment>, sqlx::Error> {
    sqlx::query_as!(
        AppointmentRow,
        r#"SELECT id, customer_id, vehicle_id, dealership_id, service_type_id,
                  bay_id, technician_id,
                  lower(slot) AS "start!", upper(slot) AS "end!",
                  status, created_at
           FROM appointment
           WHERE id = $1"#,
        id,
    )
    .fetch_optional(conn)
    .await?
    .map(Appointment::try_from)
    .transpose()
}

pub async fn cancel_appointment(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Appointment>, sqlx::Error> {
    sqlx::query_as!(
        AppointmentRow,
        r#"UPDATE appointment
           SET status = 'cancelled', updated_at = now()
           WHERE id = $1 AND status = 'confirmed'
           RETURNING id, customer_id, vehicle_id, dealership_id, service_type_id,
                     bay_id, technician_id,
                     lower(slot) AS "start!", upper(slot) AS "end!",
                     status, created_at"#,
        id,
    )
    .fetch_optional(conn)
    .await?
    .map(Appointment::try_from)
    .transpose()
}
