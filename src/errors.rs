use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("claims retrieval: {0}")]
    ClaimsRetrieval(String),
    #[error("missing email in claims")]
    MissingEmailInClaims,
    #[error("parse URI: {0}")]
    ParseURI(String),
    #[error("database: {0}")]
    Database(String),
    #[error("invalid first path segment: {0:?}")]
    InvalidFirstPathSegment(Option<String>),
    #[error("chimitheque person id header missing")]
    ChimithequePersonIdHeaderMissing,
    #[error("chimitheque person id header invalid: {0}")]
    ChimithequePersonIdHeaderInvalid(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("casbin error: {0}")]
    CasbinError(String),
    #[error("casbin enforcer lock failed: {0}")]
    CasbinEnforcerLockFailed(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::ClaimsRetrieval(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::ClaimsRetrieval(s).to_string(),
            ),
            AppError::MissingEmailInClaims => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::MissingEmailInClaims.to_string(),
            ),
            AppError::ParseURI(s) => (StatusCode::BAD_REQUEST, AppError::ParseURI(s).to_string()),
            AppError::Database(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::Database(s).to_string(),
            ),
            AppError::ChimithequePersonIdHeaderMissing => (
                StatusCode::FORBIDDEN,
                AppError::ChimithequePersonIdHeaderMissing.to_string(),
            ),
            AppError::InvalidFirstPathSegment(s) => (
                StatusCode::BAD_REQUEST,
                AppError::InvalidFirstPathSegment(s).to_string(),
            ),
            AppError::ChimithequePersonIdHeaderInvalid(header_value) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::ChimithequePersonIdHeaderInvalid(header_value).to_string(),
            ),
            AppError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                AppError::PermissionDenied.to_string(),
            ),
            AppError::CasbinError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::CasbinError(err).to_string(),
            ),
            AppError::CasbinEnforcerLockFailed(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::CasbinEnforcerLockFailed(s).to_string(),
            ),
        };
        (status, body).into_response()
    }
}
