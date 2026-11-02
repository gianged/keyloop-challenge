use chrono::{NaiveDate, TimeDelta, Utc};
use uuid::Uuid;

use crate::domain::Slot;
use crate::error::AppError;
use crate::repo;
use crate::state::AppState;

pub async fn open_slots(
    state: &AppState,
    dealership_id: Uuid,
    service_type_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<Slot>, AppError> {
    let mut conn = state.pool.acquire().await?;
    if !repo::dealership_exists(&mut conn, dealership_id).await? {
        return Err(AppError::NotFound);
    }
    let service_type = repo::find_service_type(&mut conn, service_type_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let day_start = date.and_time(chrono::NaiveTime::MIN).and_utc();
    let day_end = day_start + TimeDelta::days(1);
    let starts = repo::open_starts(
        &mut conn,
        dealership_id,
        &service_type.required_skill,
        day_start..day_end,
        state.grid.step(),
        service_type.duration,
        Utc::now(),
    )
    .await?;
    Ok(starts
        .into_iter()
        .map(|start| Slot::new(start, service_type.duration))
        .collect())
}
