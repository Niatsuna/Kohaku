use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KohakuError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("RequestTimeout: {0}")]
    RequestTimeout(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Database connection error: {0}")]
    DatabaseConnectionError(#[from] diesel::r2d2::PoolError),

    #[error("Database query error: {0}")]
    DatabaseQueryError(#[from] diesel::result::Error),

    #[error("Scheduler error: {0}")]
    SchedulerError(#[from] tokio_cron_scheduler::JobSchedulerError),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task execution error: {0}")]
    TaskExecutionError(#[from] Box<KohakuError>),

    #[error("Task timeout: {0}")]
    TaskTimeout(String),

    #[error("Websocket error: {0}")]
    WebsocketError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),
}

impl KohakuError {
    pub fn error_type(&self) -> String {
        let s = match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "NOT_FOUND",
            Self::RequestTimeout(_) => "REQUEST_TIMEOUT",
            Self::Conflict(_) => "CONFLICT",
            Self::TooManyRequests(_) => "TOO_MANY_REQUESTS",
            Self::AuthenticationError(_) => "AUTHENTICATION_ERROR",
            Self::DatabaseConnectionError(_) => "DATABASE_CONNECTION_ERROR",
            Self::DatabaseQueryError(e) => {
                let t1 = "DATABASE_QUERY_";
                let t2 = match e {
                    diesel::result::Error::DatabaseError(_, _) => "CONFLICT_",
                    diesel::result::Error::NotFound => "NOT_FOUND_",
                    _ => "",
                };
                let t3 = format!("{t1}{t2}ERROR");
                &t3.clone()
            }
            Self::SchedulerError(_) => "SCHEDULER_ERROR",
            Self::TaskNotFound(_) => "TASK_NOT_FOUND",
            Self::TaskExecutionError(_) => "TASK_EXECUTION_ERROR",
            Self::TaskTimeout(_) => "TASK_TIMEOUT",
            Self::WebsocketError(_) => "WEBSOCKET_ERROR",
            Self::ExternalServiceError(_) => "EXTERNAL_SERVICE_ERROR",
            #[allow(unreachable_patterns)]
            _ => "UNKNOWN",
        };
        s.to_string()
    }
}

impl ResponseError for KohakuError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            // 400
            Self::BadRequest(_) | Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            // 401
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            // 403
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            // 404
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            // 408
            Self::RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            // 409
            Self::Conflict(_) => StatusCode::CONFLICT,
            // 429
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            // 500
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let status = self.status_code();
        let kind = self.error_type();
        let message = if status.is_server_error() {
            // 5XX : Hide implementation details from clients
            match self {
                Self::ExternalServiceError(_) => {
                    "An external service is currently unavailable".to_string()
                }
                _ => "An internal error occured. Please try again later.".to_string(),
            }
        } else {
            // 4XX : Expose details as it is the clients fault
            self.to_string()
        };

        HttpResponse::build(status).json(serde_json::json!({
            "status" : status.as_u16(),
            "kind" : kind,
            "message" : message
        }))
    }
}
