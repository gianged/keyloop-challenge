use uuid::Uuid;

use crate::domain::Appointment;
use crate::error::AppError;
use crate::repo;
use crate::state::AppState;

pub async fn get(state: &AppState, id: Uuid) -> Result<Appointment, AppError> {
    let mut conn = state.pool.acquire().await?;
    repo::find_appointment_by_id(&mut conn, id)
        .await?
        .ok_or(AppError::NotFound)
}
