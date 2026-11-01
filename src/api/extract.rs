use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use uuid::Uuid;

use crate::error::AppError;

pub struct PathId(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for PathId {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map(|Path(id)| Self(id))
            .map_err(|_| AppError::Unprocessable("invalid_id"))
    }
}
