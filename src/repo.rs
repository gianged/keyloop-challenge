mod appointment;
mod availability;
mod resource;

pub use appointment::{
    InsertOutcome, cancel_appointment, find_appointment_by_id, find_appointment_by_key,
    insert_appointment,
};
pub use resource::{booking_refs_exist, find_service_type, free_bays, free_technicians};
