use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{Appointment, AppointmentStatus};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StatusDto {
    Confirmed,
    Cancelled,
}

impl From<AppointmentStatus> for StatusDto {
    fn from(status: AppointmentStatus) -> Self {
        match status {
            AppointmentStatus::Confirmed => Self::Confirmed,
            AppointmentStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAppointmentRequest {
    pub customer_id: Uuid,
    pub vehicle_id: Uuid,
    pub dealership_id: Uuid,
    pub service_type_id: Uuid,
    pub start: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AppointmentResponse {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub vehicle_id: Uuid,
    pub dealership_id: Uuid,
    pub service_type_id: Uuid,
    pub bay_id: Uuid,
    pub technician_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub status: StatusDto,
    pub created_at: DateTime<Utc>,
}

impl From<Appointment> for AppointmentResponse {
    fn from(a: Appointment) -> Self {
        Self {
            id: a.id,
            customer_id: a.customer_id,
            vehicle_id: a.vehicle_id,
            dealership_id: a.dealership_id,
            service_type_id: a.service_type_id,
            bay_id: a.bay_id,
            technician_id: a.technician_id,
            start: a.slot.start,
            end: a.slot.end,
            status: a.status.into(),
            created_at: a.created_at,
        }
    }
}
