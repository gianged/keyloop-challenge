use tokio::sync::broadcast::Receiver;
use uuid::Uuid;

use crate::domain::Event;
use crate::error::AppError;
use crate::repo;
use crate::state::AppState;

pub async fn subscribe(
    state: &AppState,
    dealership_id: Uuid,
) -> Result<Receiver<(Uuid, Event)>, AppError> {
    let receiver = state.bus.subscribe();
    let mut conn = state.pool.acquire().await?;
    if !repo::dealership_exists(&mut conn, dealership_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(receiver)
}
