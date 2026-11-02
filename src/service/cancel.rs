use uuid::Uuid;

use crate::domain::{Appointment, Event};
use crate::error::AppError;
use crate::repo;
use crate::state::AppState;

pub async fn cancel(state: &AppState, id: Uuid) -> Result<Appointment, AppError> {
    let mut conn = state.pool.acquire().await?;
    if let Some(cancelled) = repo::cancel_appointment(&mut conn, id).await? {
        state
            .bus
            .publish(cancelled.dealership_id, Event::cancelled(&cancelled));
        return Ok(cancelled);
    }
    repo::find_appointment_by_id(&mut conn, id)
        .await?
        .ok_or(AppError::NotFound)
}
