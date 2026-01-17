use actix_web::{body::to_bytes, http::StatusCode, ResponseError};
use rstest::rstest;
use serde::Deserialize;

use crate::utils::error::KohakuError;

// Missing tests for:
// - KohakuError::DatabaseConnectionError
// - KohakuError::DatabaseQueryError(diesel::result::Error) if not diesel::result::Error::NotFound
// Issue: Construction of said error types

#[rstest]
#[case(KohakuError::BadRequest("".into()), "BAD_REQUEST")]
#[case(KohakuError::ValidationError("".into()), "VALIDATION_ERROR")]
#[case(KohakuError::Unauthorized("".into()), "UNAUTHORIZED")]
#[case(KohakuError::Forbidden("".into()), "FORBIDDEN")]
#[case(KohakuError::NotFound("".into()), "NOT_FOUND")]
#[case(KohakuError::RequestTimeout("".into()), "REQUEST_TIMEOUT")]
#[case(KohakuError::Conflict("".into()), "CONFLICT")]
#[case(KohakuError::TooManyRequests("".into()), "TOO_MANY_REQUESTS")]
#[case(KohakuError::AuthenticationError("".into()), "AUTHENTICATION_ERROR")]
#[case(
    KohakuError::DatabaseQueryError(diesel::result::Error::NotFound),
    "DATABASE_QUERY_NOT_FOUND_ERROR"
)]
#[case(
    KohakuError::SchedulerError(tokio_cron_scheduler::JobSchedulerError::CantInit),
    "SCHEDULER_ERROR"
)]
#[case(KohakuError::TaskBuilderError("".into()), "TASK_BUILDER_ERROR")]
#[case(KohakuError::TaskNotFound("".into()), "TASK_NOT_FOUND")]
#[case(KohakuError::TaskExecutionError(Box::new(KohakuError::BadRequest("".into()))), "TASK_EXECUTION_ERROR")]
#[case(KohakuError::TaskTimeout("".into()), "TASK_TIMEOUT")]
#[case(KohakuError::WebsocketError("".into()), "WEBSOCKET_ERROR")]
#[case(KohakuError::ExternalServiceError("".into()), "EXTERNAL_SERVICE_ERROR")]
fn test_kohakuerror_error_type(#[case] error: KohakuError, #[case] expected_kind: &str) {
    let s = expected_kind.to_string();
    assert_eq!(error.error_type(), s);
}

#[rstest]
#[case(KohakuError::BadRequest("".into()), StatusCode::BAD_REQUEST)]
#[case(KohakuError::ValidationError("".into()), StatusCode::BAD_REQUEST)]
#[case(KohakuError::Unauthorized("".into()), StatusCode::UNAUTHORIZED)]
#[case(KohakuError::Forbidden("".into()), StatusCode::FORBIDDEN)]
#[case(KohakuError::NotFound("".into()), StatusCode::NOT_FOUND)]
#[case(KohakuError::RequestTimeout("".into()), StatusCode::REQUEST_TIMEOUT)]
#[case(KohakuError::Conflict("".into()), StatusCode::CONFLICT)]
#[case(KohakuError::TooManyRequests("".into()), StatusCode::TOO_MANY_REQUESTS)]
#[case(KohakuError::AuthenticationError("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(
    KohakuError::DatabaseQueryError(diesel::result::Error::NotFound),
    StatusCode::INTERNAL_SERVER_ERROR
)]
#[case(
    KohakuError::SchedulerError(tokio_cron_scheduler::JobSchedulerError::CantInit),
    StatusCode::INTERNAL_SERVER_ERROR
)]
#[case(KohakuError::TaskBuilderError("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(KohakuError::TaskNotFound("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(KohakuError::TaskExecutionError(Box::new(KohakuError::BadRequest("".into()))), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(KohakuError::TaskTimeout("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(KohakuError::WebsocketError("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
#[case(KohakuError::ExternalServiceError("".into()), StatusCode::INTERNAL_SERVER_ERROR)]
fn test_kohakuerror_status_code(#[case] error: KohakuError, #[case] expected_status: StatusCode) {
    assert_eq!(error.status_code(), expected_status);
}

#[derive(Debug, Deserialize, PartialEq)]
struct ErrorJson {
    status: u16,
    kind: String,
    message: String,
}

#[tokio::test]
#[rstest]
#[case(KohakuError::BadRequest("".into()))]
#[case(KohakuError::ValidationError("".into()))]
#[case(KohakuError::Unauthorized("".into()))]
#[case(KohakuError::Forbidden("".into()))]
#[case(KohakuError::NotFound("".into()))]
#[case(KohakuError::RequestTimeout("".into()))]
#[case(KohakuError::Conflict("".into()))]
#[case(KohakuError::TooManyRequests("".into()))]
#[case(KohakuError::AuthenticationError("".into()))]
#[case(KohakuError::DatabaseQueryError(diesel::result::Error::NotFound))]
#[case(KohakuError::SchedulerError(tokio_cron_scheduler::JobSchedulerError::CantInit))]
#[case(KohakuError::TaskBuilderError("".into()))]
#[case(KohakuError::TaskNotFound("".into()))]
#[case(KohakuError::TaskExecutionError(Box::new(KohakuError::BadRequest("".into()))))]
#[case(KohakuError::TaskTimeout("".into()))]
#[case(KohakuError::WebsocketError("".into()))]
#[case(KohakuError::ExternalServiceError("".into()))]
async fn test_kohakuerror_response(#[case] error: KohakuError) {
    let status = error.status_code();
    let kind = error.error_type();
    let message = if status.is_server_error() {
        // 5XX : Hide implementation details from clients
        match error {
            KohakuError::ExternalServiceError(_) => {
                "An external service is currently unavailable".to_string()
            }
            _ => "An internal error occured. Please try again later.".to_string(),
        }
    } else {
        // 4XX : Expose details as it is the clients fault
        error.to_string()
    };
    let response = error.error_response();

    assert_eq!(response.status(), status);

    let body = to_bytes(response.into_body()).await.unwrap();
    let json: ErrorJson = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json,
        ErrorJson {
            status: status.as_u16(),
            kind,
            message
        }
    )
}
