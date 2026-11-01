use axum::Json;
use axum::extract::State;

use super::dto::{AvailabilityQuery, SlotResponse};
use super::extract::{PathId, ValidQuery};
use crate::error::{AppError, ErrorBody};
use crate::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/dealerships/{id}/availability",
    params(
        ("id" = Uuid, Path, description = "Dealership id"),
        AvailabilityQuery,
    ),
    responses(
        (status = 200, description = "Open start times for the UTC day, empty when none", body = Vec<SlotResponse>),
        (status = 404, description = "Unknown dealership or service type", body = ErrorBody),
        (status = 422, description = "Malformed id, service_type_id or date", body = ErrorBody),
        (status = 503, description = "Database unreachable", body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    PathId(id): PathId,
    ValidQuery(query): ValidQuery<AvailabilityQuery>,
) -> Result<Json<Vec<SlotResponse>>, AppError> {
    let slots = service::open_slots(&state, id, query.service_type_id, query.date).await?;
    Ok(Json(slots.into_iter().map(Into::into).collect()))
}
