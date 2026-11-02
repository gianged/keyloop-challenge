use uuid::Uuid;

use super::appointment::Appointment;
use super::slot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Booked,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub kind: EventKind,
    pub appointment_id: Uuid,
    pub slot: Slot,
}

impl Event {
    pub fn booked(appointment: &Appointment) -> Self {
        Self::of(EventKind::Booked, appointment)
    }

    pub fn cancelled(appointment: &Appointment) -> Self {
        Self::of(EventKind::Cancelled, appointment)
    }

    fn of(kind: EventKind, appointment: &Appointment) -> Self {
        Self {
            kind,
            appointment_id: appointment.id,
            slot: appointment.slot,
        }
    }
}
