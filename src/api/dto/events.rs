use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::Event;

#[derive(Serialize, ToSchema)]
pub struct AppointmentEventPayload {
    pub appointment_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl From<Event> for AppointmentEventPayload {
    fn from(event: Event) -> Self {
        Self {
            appointment_id: event.appointment_id,
            start: event.slot.start,
            end: event.slot.end,
        }
    }
}
