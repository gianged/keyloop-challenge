use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use futures_core::Stream;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use super::dto::AppointmentEventPayload;
use super::extract::PathId;
use crate::domain::{Event, EventKind};
use crate::error::{AppError, ErrorBody};
use crate::service;
use crate::state::AppState;

const KEEP_ALIVE: Duration = Duration::from_secs(15);

#[utoipa::path(
    get,
    path = "/dealerships/{id}/events",
    params(("id" = Uuid, Path, description = "Dealership id")),
    responses(
        (
            status = 200,
            description = "Event stream. `booked` and `cancelled` carry the payload, `resync` carries `{}` after the subscriber fell behind. Refetch availability on each, and on reconnect",
            body = AppointmentEventPayload,
            content_type = "text/event-stream"
        ),
        (status = 404, description = "Unknown dealership", body = ErrorBody),
        (status = 422, description = "Malformed id", body = ErrorBody),
        (status = 503, description = "Database unreachable", body = ErrorBody),
    )
)]
pub async fn stream(
    State(state): State<AppState>,
    PathId(dealership_id): PathId,
) -> Result<Sse<impl Stream<Item = Result<SseEvent, axum::Error>>>, AppError> {
    let mut receiver = service::subscribe(&state, dealership_id).await?;
    let shutdown = state.shutdown.clone();

    let events = async_stream::stream! {
        tracing::info!(%dealership_id, "sse connected");
        let _connection = Connection(dealership_id);
        loop {
            let received = tokio::select! {
                () = shutdown.cancelled() => break,
                received = receiver.recv() => received,
            };
            match received {
                Ok((id, event)) if id == dealership_id => yield to_sse(event),
                Ok(_) => {}
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(%dealership_id, skipped, "sse subscriber lagged");
                    yield Ok(SseEvent::default().event("resync").data("{}"));
                }
                Err(RecvError::Closed) => break,
            }
        }
    };
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)))
}

struct Connection(Uuid);

impl Drop for Connection {
    fn drop(&mut self) {
        tracing::info!(dealership_id = %self.0, "sse disconnected");
    }
}

fn to_sse(event: Event) -> Result<SseEvent, axum::Error> {
    let name = match event.kind {
        EventKind::Booked => "booked",
        EventKind::Cancelled => "cancelled",
    };
    SseEvent::default()
        .event(name)
        .json_data(AppointmentEventPayload::from(event))
}
