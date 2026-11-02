mod availability;
mod booking;
mod cancel;
mod lookup;

pub use availability::open_slots;
pub use booking::{Booking, Outcome, book};
pub use cancel::cancel;
pub use lookup::get;
