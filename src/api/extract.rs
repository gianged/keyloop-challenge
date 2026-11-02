use axum::extract::{FromRequestParts, Path, Query};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
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

pub struct ValidQuery<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for ValidQuery<T> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|_| AppError::Unprocessable("invalid_query"))
    }
}
