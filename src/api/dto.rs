mod appointment;
mod availability;
mod events;

pub use appointment::{AppointmentResponse, CreateAppointmentRequest};
pub use availability::{AvailabilityQuery, SlotResponse};
pub use events::AppointmentEventPayload;
