# Error Handling
Error handling is provided via a custom enum called `KohakuError` in [`error.rs`](../server/src/utils/error.rs) to wrap and map existing errors from other crates and construct custom errors while providing means to map these errors to meaningful HTTP status codes after the [`RFC 9110`]() definition:

- `2XX` - Successful transaction
- `4XX` - Client errors: User can fix by changing their request
- `5XX` - Server errors: System issue that the user cannot fix

Each response is in the structure of a JSON format, while errors feature the following structure:
```json
{
  "status": HTTP_STATUS_CODE,
  "kind": KOHAKU_ERROR_NAME,
  "message": ERROR_MESSAGE,
}
```
to provide meaningful ways to differentiate the response from the server.

## Errors
Errors can either happen during requests from a client or in internally defined processes like tasks, services or other core features.

If an error occurs during a request, the error should be propagated to the response the client receives.
While client errors should be probably exposed, server errors should hide the implementation for safety purposes.

If the error occurs in a internally server-sided process, the error should be logged to make debugging more viable.

### Error Types
The following table shows what kind of errors are currently available on the server-side:

| Name | Description | Status | Example encounter |
| ---- | ----------- | ------ | ----------------- |
| `BadRequest` | Given input is syntactically incorrect (e.g. results in a malformed JSON, wrong types) | `400` |
| `ValidationError` | Given input is semantic incorrect and violates business logic (e.g. only positive numbers allowed but got -1 ) | `400` | [JWT generation](../server/src/utils/comm/auth/jwt.rs#L51)
| `Unauthorized` | Authorization failed, API key or token is invalid | `401` | [Authentication check](../server/src/utils/comm/auth/mod.rs#L62)
| `Forbidden` | Missing permissions after successful authorization | `403` | [Authentication check](../server/src/utils/comm/auth/mod.rs#L62)
| `NotFound` | Requested resource not found | `404` |
| `RequestTimeout` | Response would be sent on an idle / inactive connection | `408` |
| `Conflict` | Requested transaction violates any underlaying constraints that do not fall under `ValidationError`s business logic (e.g. unique constraint in database entry) | `409` |
| `TooManyRequests` | Limit for requests in a timeframe reached | `429` |
| 
| `AuthenticationError` | Indicates that some process during authentication itself failed (e.g. hashing failed because of a invalid salt, JWTService failed to start); Maps: [`argon2::Error`](https://docs.rs/rust-argon2/latest/argon2/enum.Error.html) | `500` | [Hashing of API keys](../server/src/utils/comm/auth/api_key.rs#L73)
| `DatabaseConnectionError` | Database connection is either invalid, closed or failed; Wraps: [`diesel::r2d2::PoolError`](https://docs.diesel.rs/master/diesel/r2d2/type.PoolError.html) | `500` | [Database Connection](../server/src/db/mod.rs#L47)
| `DatabaseQueryError` | Database query failed to execute; Wraps: [`diesel::result::Error`](https://docs.diesel.rs/2.1.x/diesel/result/enum.Error.html) | `500` | [Storing API keys in Database](../server/src/utils/comm/auth/models.rs#L80)
| `SchedulerError` | Initialization or task scheduling failed; Wraps: [`tokio_cron_scheduler::JobSchedulerError`](https://docs.rs/tokio-cron-scheduler/latest/tokio_cron_scheduler/enum.JobSchedulerError.html) | `500` | [Scheduler Start](../server/src/utils/scheduler/mod.rs#L56)
| `TaskNotFound` | Scheduled task cannot be found (Sync issue) | `500` |
| `TaskExecutionError` | Failed to execute the given task, an error occured during the task | `500` |
| `TaskTimeout` | Scheduled task timeout | `500` |
| `WebsockertError` | Indicating some occuring error in the websocket communication module (e.g. manager failed to start) | `500` | [Websocket Endpoint](../server/src/utils/comm/websocket/routes.rs#L13)
| `ExternalServiceError` | External service (e.g. another API) returned an error | `500` | [Database migration at startup](../server/src/db/mod.rs#L52)
