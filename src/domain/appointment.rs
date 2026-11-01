use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::slot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppointmentStatus {
    Confirmed,
    Cancelled,
}

impl AppointmentStatus {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "confirmed" => Some(Self::Confirmed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BookingRequest {
    pub customer_id: Uuid,
    pub vehicle_id: Uuid,
    pub dealership_id: Uuid,
    pub service_type_id: Uuid,
    pub start: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Clone, Debug)]
pub struct Appointment {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub vehicle_id: Uuid,
    pub dealership_id: Uuid,
    pub service_type_id: Uuid,
    pub bay_id: Uuid,
    pub technician_id: Uuid,
    pub slot: Slot,
    pub status: AppointmentStatus,
    pub created_at: DateTime<Utc>,
}

impl Appointment {
    pub fn matches(&self, req: &BookingRequest) -> bool {
        self.customer_id == req.customer_id
            && self.vehicle_id == req.vehicle_id
            && self.dealership_id == req.dealership_id
            && self.service_type_id == req.service_type_id
            && self.slot.start == req.start
    }
}
