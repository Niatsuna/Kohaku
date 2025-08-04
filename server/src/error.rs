use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    pub code: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum KohakuError {
    //TODO: Rework when database connection is implemented
    #[error("A database error occured: {0}")]
    DatabaseError(String),

    #[error("External API error from {service} ({status}): {message}")]
    ExternalApiError {
        service: String,
        status: u16,
        message: String,
    },

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded for {service}!")]
    RateLimitExceeded {
        service: String,
        retry_after: Option<u64>,
    },

    #[error("Failed to scrape data from {service}: {message}")]
    ScrapingError { service: String, message: String },
}

impl KohakuError {
    /// Returns the HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match self {
            KohakuError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            KohakuError::ExternalApiError {
                service,
                status,
                message,
            } => StatusCode::BAD_GATEWAY,
            KohakuError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            KohakuError::NotFound(_) => StatusCode::NOT_FOUND,
            KohakuError::RateLimitExceeded {
                service,
                retry_after,
            } => StatusCode::TOO_MANY_REQUESTS,
            KohakuError::ScrapingError { service, message } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn details(&self) -> Option<Value> {
        match self {
            KohakuError::ExternalApiError {
                service,
                status,
                message,
            } => Some(serde_json::json!({
              "service": service,
              "status": status,
              "message": message
            })),
            KohakuError::RateLimitExceeded {
                service,
                retry_after,
            } => {
                if let Some(retry) = retry_after {
                    Some(serde_json::json!({"service" : service, "retry_after": retry}))
                } else {
                    Some(serde_json::json!({"service": service}))
                }
            }
            KohakuError::ScrapingError { service, message } => {
                Some(serde_json::json!({"service": service, "message": message}))
            }
            _ => None,
        }
    }
}

impl ResponseError for KohakuError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorResponse {
            message: self.to_string(),
            code: self.status_code().as_u16(),
            details: self.details(),
        })
    }
}
