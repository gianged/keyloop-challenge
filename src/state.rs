use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::domain::SlotGrid;
use crate::infra::{EventBus, Locks};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub locks: Locks,
    pub grid: SlotGrid,
    pub bus: EventBus,
    pub shutdown: CancellationToken,
}
