use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: &'static str,
}

pub enum AppError {
    Db(sqlx::Error),
    NotFound,
    Conflict(&'static str),
    Unprocessable(&'static str),
    LockTimeout,
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Db(err) if db_unavailable(&err) => {
                tracing::error!(error = %err, "database unavailable");
                respond(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
            Self::Db(err) => {
                tracing::error!(error = %err, "database error");
                respond(StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
            Self::LockTimeout => {
                tracing::warn!("dealership lock timeout");
                respond(StatusCode::SERVICE_UNAVAILABLE, "busy")
            }
            Self::NotFound => respond(StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(code) => respond(StatusCode::CONFLICT, code),
            Self::Unprocessable(code) => respond(StatusCode::UNPROCESSABLE_ENTITY, code),
        }
    }
}

fn db_unavailable(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_)
    )
}

fn respond(status: StatusCode, code: &'static str) -> Response {
    (status, Json(ErrorBody { code })).into_response()
}
