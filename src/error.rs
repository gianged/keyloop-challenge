use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum AppError {
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err)
    }
}

fn db_unavailable(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_)
    )
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Db(err) if db_unavailable(&err) => {
                tracing::error!(error = %err, "database unavailable");
                (StatusCode::SERVICE_UNAVAILABLE, "service unavailable").into_response()
            }
            Self::Db(err) => {
                tracing::error!(error = %err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}
