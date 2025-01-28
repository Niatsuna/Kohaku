use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KohakuError {
    // Database Errors
    #[error("Connection pool error: {0}")]
    ConnectionPoolError(#[from] r2d2::Error),

    #[error("Query result error: {0}")]
    QueryResultError(#[from] diesel::result::Error),

    // Scraper Errors
    #[error("Parse error (Time): {0}")]
    ParseTimeError(#[from] chrono::format::ParseError),

    #[error("Parse error (Str): {0}")]
    ParseStrError(#[from] reqwest::header::ToStrError),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Serialization error (serde): {0}")]
    SerdeError(#[from] serde_json::Error),

    // Handler Errors
    #[error("Validation error: {0}")]
    ValidationError(#[from] validator::ValidationErrors),

    // Others
    #[error("Kohaku Custom Error: {0}")]
    CustomError(String),
}

#[allow(unreachable_patterns)]
impl ResponseError for KohakuError {
    fn error_response(&self) -> HttpResponse {
        let msg = self.to_string();
        match self {
            KohakuError::ConnectionPoolError(_) => HttpResponse::ServiceUnavailable().body(msg),
            KohakuError::QueryResultError(_) => HttpResponse::InternalServerError().body(msg),
            KohakuError::ParseTimeError(_) => HttpResponse::BadRequest().body(msg),
            KohakuError::ParseStrError(_) => HttpResponse::BadRequest().body(msg),
            KohakuError::RequestError(_) => HttpResponse::BadGateway().body(msg),
            KohakuError::SerdeError(_) => HttpResponse::UnprocessableEntity().body(msg),
            KohakuError::ValidationError(_) => HttpResponse::BadRequest().body(msg),
            _ => HttpResponse::InternalServerError().body(msg),
        }
    }
}
