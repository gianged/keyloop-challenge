use axum::extract::State;
use axum::http::StatusCode;

use crate::error::AppError;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "Database reachable"),
        (status = 503, description = "Database unreachable"),
    )
)]
pub async fn healthz(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(StatusCode::OK)
}
