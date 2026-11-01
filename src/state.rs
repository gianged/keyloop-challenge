use sqlx::PgPool;

use crate::domain::SlotGrid;
use crate::infra::Locks;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub locks: Locks,
    pub grid: SlotGrid,
}
