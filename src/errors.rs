use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("missing email in claims")]
    MissingEmailInClaims,
    #[error("parse URI: {0}")]
    ParseURI(String),
    #[error("parse email: {0}")]
    ParseEmail(String),
    #[error("database: {0}")]
    Database(String),
    #[error("database pool: {0}")]
    DatabasePool(String),
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
    #[error("invalid person: {0}")]
    InvalidPerson(String),
    #[error("invalid product: {0}")]
    InvalidProduct(String),
    #[error("input validation error: {0}")]
    InputValidation(String),
    #[error("pubchem error: {0}")]
    Pubchem(String),
    #[error("bearer token missing error")]
    BearerTokenMissing,
    #[error("decode jwt header error: {0}")]
    DecodeJWTHeader(String),
    #[error("header kid missing")]
    HeaderKIDMissing,
    #[error("rsa jwk not found in cache with kid {0}")]
    RSAJWKNotFoundInCache(String),
    #[error("claims decoding: {0}")]
    ClaimsDecoding(String),
    #[error("certificates retrieval error: {0}")]
    CertificatesRetrieval(String),
    #[error("decode jwks error: {0}")]
    DecodeJWKS(String),
    #[error("refresh jwks error: {0}")]
    RefreshJWKS(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::ClaimsDecoding(s) => {
                error!("ClaimsDecoding: {}", s);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::ClaimsDecoding(s).to_string(),
                )
            }
            AppError::MissingEmailInClaims => {
                error!("MissingEmailInClaims");
                (
                    StatusCode::BAD_REQUEST,
                    AppError::MissingEmailInClaims.to_string(),
                )
            }
            AppError::ParseURI(s) => {
                error!("ParseURI: {}", s);
                (StatusCode::BAD_REQUEST, AppError::ParseURI(s).to_string())
            }
            AppError::ParseEmail(s) => {
                error!("ParseEmail: {}", s);
                (StatusCode::BAD_REQUEST, AppError::ParseEmail(s).to_string())
            }
            AppError::Database(s) => {
                error!("Database: {}", s);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppError::Database(s).to_string(),
                )
            }
            AppError::DatabasePool(s) => {
                error!("DatabasePool: {}", s);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppError::DatabasePool(s).to_string(),
                )
            }
            AppError::ChimithequePersonIdHeaderMissing => {
                error!("ChimithequePersonIdHeaderMissing");
                (
                    StatusCode::FORBIDDEN,
                    AppError::ChimithequePersonIdHeaderMissing.to_string(),
                )
            }
            AppError::InvalidFirstPathSegment(s) => {
                error!("InvalidFirstPathSegment: {:?}", s);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::InvalidFirstPathSegment(s).to_string(),
                )
            }
            AppError::ChimithequePersonIdHeaderInvalid(header_value) => {
                error!("ChimithequePersonIdHeaderInvalid: {}", header_value);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::ChimithequePersonIdHeaderInvalid(header_value).to_string(),
                )
            }
            AppError::PermissionDenied => {
                // We do not log permission denied errors.
                (
                    StatusCode::FORBIDDEN,
                    AppError::PermissionDenied.to_string(),
                )
            }
            AppError::CasbinError(err) => {
                error!("CasbinError: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppError::CasbinError(err).to_string(),
                )
            }
            AppError::CasbinEnforcerLockFailed(s) => {
                error!("CasbinEnforcerLockFailed: {}", s);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppError::CasbinEnforcerLockFailed(s).to_string(),
                )
            }
            AppError::InvalidPerson(s) => {
                error!("InvalidPerson: {}", s);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::InvalidPerson(s).to_string(),
                )
            }
            AppError::InvalidProduct(s) => {
                error!("InvalidProduct: {}", s);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::InvalidProduct(s).to_string(),
                )
            }
            AppError::InputValidation(s) => {
                error!("InputValidation: {}", s);
                (
                    StatusCode::BAD_REQUEST,
                    AppError::InputValidation(s).to_string(),
                )
            }
            AppError::Pubchem(s) => {
                error!("Pubchem: {}", s);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppError::Pubchem(s).to_string(),
                )
            }
            AppError::BearerTokenMissing => {
                error!("BearerTokenMissing");
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::BearerTokenMissing.to_string(),
                )
            }
            AppError::DecodeJWTHeader(s) => {
                error!("DecodeJWTHeader: {}", s);
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::DecodeJWTHeader(s).to_string(),
                )
            }
            AppError::HeaderKIDMissing => {
                error!("HeaderKIDMissing");
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::HeaderKIDMissing.to_string(),
                )
            }
            AppError::RSAJWKNotFoundInCache(kid) => {
                error!("RSAJWKNotFoundInCache: {}", kid);
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::RSAJWKNotFoundInCache(kid).to_string(),
                )
            }
            AppError::CertificatesRetrieval(s) => {
                error!("CertificatesRetrieval: {}", s);
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::CertificatesRetrieval(s).to_string(),
                )
            }
            AppError::DecodeJWKS(s) => {
                error!("DecodeJWKS: {}", s);
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::DecodeJWKS(s).to_string(),
                )
            }
            AppError::RefreshJWKS(s) => {
                error!("RefreshJWKS: {}", s);
                (
                    StatusCode::UNAUTHORIZED,
                    AppError::RefreshJWKS(s).to_string(),
                )
            }
        };
        (status, body).into_response()
    }
}
