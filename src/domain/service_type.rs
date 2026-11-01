use chrono::TimeDelta;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ServiceType {
    pub id: Uuid,
    pub duration: TimeDelta,
    pub required_skill: String,
}
