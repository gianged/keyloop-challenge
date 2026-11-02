use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};

use super::dto::{AppointmentResponse, CreateAppointmentRequest};
use super::extract::PathId;
use crate::domain::BookingRequest;
use crate::error::{AppError, ErrorBody};
use crate::service::{self, Outcome};
use crate::state::AppState;

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const IDEMPOTENCY_KEY_MAX_LEN: usize = 255;

#[utoipa::path(
    post,
    path = "/appointments",
    params(
        ("Idempotency-Key" = String, Header, description = "Client-chosen key, unique per customer. Replays return the original appointment")
    ),
    request_body = CreateAppointmentRequest,
    responses(
        (status = 201, description = "Booked", body = AppointmentResponse),
        (status = 200, description = "Idempotent replay of an earlier booking", body = AppointmentResponse),
        (status = 409, description = "No bay and qualified technician free, or vehicle already booked", body = ErrorBody),
        (status = 422, description = "Invalid start, unknown reference, bad or reused Idempotency-Key", body = ErrorBody),
        (status = 503, description = "Database unreachable or dealership busy", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateAppointmentRequest>,
) -> Result<(StatusCode, Json<AppointmentResponse>), AppError> {
    let req = BookingRequest {
        customer_id: body.customer_id,
        vehicle_id: body.vehicle_id,
        dealership_id: body.dealership_id,
        service_type_id: body.service_type_id,
        start: body.start,
        idempotency_key: idempotency_key(&headers)?,
    };
    let booked = service::book(&state, req).await?;
    let status = match booked.outcome {
        Outcome::Created => StatusCode::CREATED,
        Outcome::Replay => StatusCode::OK,
    };
    Ok((status, Json(booked.appointment.into())))
}

#[utoipa::path(
    get,
    path = "/appointments/{id}",
    params(("id" = Uuid, Path, description = "Appointment id")),
    responses(
        (status = 200, description = "Appointment", body = AppointmentResponse),
        (status = 404, description = "Unknown appointment", body = ErrorBody),
        (status = 422, description = "Malformed id", body = ErrorBody),
        (status = 503, description = "Database unreachable", body = ErrorBody),
    )
)]
pub async fn get(
    State(state): State<AppState>,
    PathId(id): PathId,
) -> Result<Json<AppointmentResponse>, AppError> {
    Ok(Json(service::get(&state, id).await?.into()))
}

#[utoipa::path(
    post,
    path = "/appointments/{id}/cancel",
    params(("id" = Uuid, Path, description = "Appointment id")),
    responses(
        (status = 200, description = "Cancelled, or already cancelled", body = AppointmentResponse),
        (status = 404, description = "Unknown appointment", body = ErrorBody),
        (status = 422, description = "Malformed id", body = ErrorBody),
        (status = 503, description = "Database unreachable", body = ErrorBody),
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    PathId(id): PathId,
) -> Result<Json<AppointmentResponse>, AppError> {
    Ok(Json(service::cancel(&state, id).await?.into()))
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|key| !key.is_empty() && key.len() <= IDEMPOTENCY_KEY_MAX_LEN)
        .map(str::to_owned)
        .ok_or(AppError::Unprocessable("invalid_idempotency_key"))
}
