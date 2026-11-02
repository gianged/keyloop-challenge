mod appointment;
mod event;
mod service_type;
mod slot;

pub use appointment::{Appointment, AppointmentStatus, BookingRequest};
pub use event::{Event, EventKind};
pub use service_type::ServiceType;
pub use slot::{Slot, SlotGrid};
