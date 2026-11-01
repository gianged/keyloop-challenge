use std::time::Instant;

use chrono::Utc;
use sqlx::{PgConnection, PgPool};

use crate::domain::{Appointment, BookingRequest, Slot, SlotGrid};
use crate::error::AppError;
use crate::repo::{self, InsertOutcome};
use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Created,
    Replay,
}

impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Replay => "replay",
        }
    }
}

pub struct Booking {
    pub appointment: Appointment,
    pub outcome: Outcome,
}

fn validate_start(grid: SlotGrid, req: &BookingRequest) -> Result<(), AppError> {
    if !grid.is_aligned(req.start) {
        return Err(AppError::Unprocessable("start_off_grid"));
    }
    if req.start < Utc::now() {
        return Err(AppError::Unprocessable("start_in_past"));
    }
    Ok(())
}

async fn find_replay(
    conn: &mut PgConnection,
    req: &BookingRequest,
) -> Result<Option<Booking>, AppError> {
    let Some(existing) =
        repo::find_appointment_by_key(conn, req.customer_id, &req.idempotency_key).await?
    else {
        return Ok(None);
    };
    if !existing.matches(req) {
        return Err(AppError::Unprocessable("idempotency_key_reused"));
    }
    Ok(Some(Booking {
        appointment: existing,
        outcome: Outcome::Replay,
    }))
}

async fn place(
    pool: &PgPool,
    req: &BookingRequest,
    attempts: &mut u32,
) -> Result<Booking, AppError> {
    let mut conn = pool.acquire().await?;

    if let Some(replay) = find_replay(&mut conn, req).await? {
        return Ok(replay);
    }

    let service_type = repo::find_service_type(&mut conn, req.service_type_id)
        .await?
        .ok_or(AppError::Unprocessable("unknown_service_type"))?;
    let refs_exist = repo::booking_refs_exist(
        &mut conn,
        req.customer_id,
        req.vehicle_id,
        req.dealership_id,
    )
    .await?;
    if !refs_exist {
        return Err(AppError::Unprocessable("unknown_reference"));
    }

    let slot = Slot::new(req.start, service_type.duration);
    let bays = repo::free_bays(&mut conn, req.dealership_id, slot).await?;
    let technicians = repo::free_technicians(
        &mut conn,
        req.dealership_id,
        &service_type.required_skill,
        slot,
    )
    .await?;

    for &bay_id in &bays {
        for &technician_id in &technicians {
            *attempts += 1;
            match repo::insert_appointment(&mut conn, req, slot, bay_id, technician_id).await? {
                InsertOutcome::Created(appointment) => {
                    return Ok(Booking {
                        appointment,
                        outcome: Outcome::Created,
                    });
                }
                InsertOutcome::BayTaken => break,
                InsertOutcome::TechnicianTaken => continue,
                InsertOutcome::VehicleTaken => {
                    return Err(AppError::Conflict("vehicle_unavailable"));
                }
                InsertOutcome::KeyExists => {
                    return find_replay(&mut conn, req)
                        .await?
                        .ok_or_else(|| sqlx::Error::RowNotFound.into());
                }
            }
        }
    }
    Err(AppError::Conflict("slot_unavailable"))
}

#[tracing::instrument(skip_all, fields(dealership_id = %req.dealership_id))]
pub async fn book(state: &AppState, req: BookingRequest) -> Result<Booking, AppError> {
    validate_start(state.grid, &req)?;

    let waiting = Instant::now();
    let _guard = state
        .locks
        .acquire(req.dealership_id)
        .await
        .map_err(|_| AppError::LockTimeout)?;
    let lock_wait_ms = waiting.elapsed().as_millis();

    let working = Instant::now();
    let mut attempts = 0;
    let result = place(&state.pool, &req, &mut attempts).await;
    let tx_ms = working.elapsed().as_millis();

    let outcome = match &result {
        Ok(booking) => Some(booking.outcome.label()),
        Err(AppError::Conflict(_)) => Some("unavailable"),
        Err(_) => None,
    };
    if let Some(outcome) = outcome {
        tracing::info!(lock_wait_ms, tx_ms, attempts, outcome, "booking finished");
    }
    result
}
