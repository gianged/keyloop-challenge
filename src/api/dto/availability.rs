use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::Slot;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AvailabilityQuery {
    pub service_type_id: Uuid,
    /// UTC day, YYYY-MM-DD
    pub date: NaiveDate,
}

#[derive(Serialize, ToSchema)]
pub struct SlotResponse {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl From<Slot> for SlotResponse {
    fn from(slot: Slot) -> Self {
        Self {
            start: slot.start,
            end: slot.end,
        }
    }
}
