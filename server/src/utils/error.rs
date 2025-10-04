use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KohakuError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("Database connection error: {0}")]
    DatabaseConnectionError(#[from] diesel::r2d2::PoolError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Operation error during {operation}: {source}")]
    OperationError {
        operation: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl KohakuError {
    fn details(&self) -> (String, StatusCode) {
        let (message, status) = match self {
            KohakuError::DatabaseConnectionError(_) => (
                "Service temporarily unavailable".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            KohakuError::ExternalServiceError(_) => (
                "External service error".to_string(),
                StatusCode::BAD_GATEWAY,
            ),

            // Propagate message
            KohakuError::NotFound(msg) => (msg.clone(), StatusCode::NOT_FOUND),
            KohakuError::ValidationError(msg) => (msg.clone(), StatusCode::BAD_REQUEST),
            KohakuError::Unauthorized(msg) => (msg.clone(), StatusCode::UNAUTHORIZED),

            // Default
            _ => (
                "Internal server error".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        };

        (message, status)
    }
}

impl ResponseError for KohakuError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let (message, status) = self.details();

        HttpResponse::build(status).json(serde_json::json!({
          "error": message,
          "status": status.as_u16()
        }))
    }

    fn status_code(&self) -> StatusCode {
        let (_, status) = self.details();

        status
    }
}
