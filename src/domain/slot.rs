use chrono::{DateTime, TimeDelta, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Slot {
    pub fn new(start: DateTime<Utc>, duration: TimeDelta) -> Self {
        Self {
            start,
            end: start + duration,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SlotGrid {
    step_seconds: i64,
}

impl SlotGrid {
    pub fn new(step_minutes: u32) -> Self {
        Self {
            step_seconds: i64::from(step_minutes) * 60,
        }
    }

    pub fn is_aligned(&self, start: DateTime<Utc>) -> bool {
        start.timestamp_subsec_nanos() == 0 && start.timestamp().rem_euclid(self.step_seconds) == 0
    }
}
