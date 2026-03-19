use actix_web::{web, HttpRequest, HttpResponse};

use crate::utils::{
    comm::{
        auth::check_authorization_token,
        events::models::{
            create_subscription, delete_subscription, get_all_topics, CreateSubscription,
            DeleteSubscription,
        },
    },
    error::KohakuError,
};

pub mod dispatcher;
pub mod models;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/topics", web::get().to(get_topics));
}

/// Get all available topics (Endpoint)
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`HttpResponse`] with status `200` which holds a list of all available topics
/// - [`Err`] : A [`KohakuError`] based on the failed operation
async fn get_topics() -> Result<HttpResponse, KohakuError> {
    let topics = get_all_topics().await?;
    Ok(HttpResponse::Ok().json(topics))
}

/// Subscribes to a given topic (Endpoint)
async fn subscribe(
    req: HttpRequest,
    body: web::Json<CreateSubscription>,
) -> Result<HttpResponse, KohakuError> {
    let claims = check_authorization_token(&req, Some(vec!["events:subscribe"])).await?;

    let topic = body.topic.clone();
    let key_id = claims.key_id;
    let target_data = body.target_data.clone();
    let sub = create_subscription(topic, key_id, target_data).await?;

    Ok(HttpResponse::Ok().json(sub))
}

/// Unsubscribe from a given topic
async fn unsubscribe(
    req: HttpRequest,
    body: web::Json<DeleteSubscription>,
) -> Result<HttpResponse, KohakuError> {
    let claims = check_authorization_token(&req, Some(vec!["events:subscribe"])).await?;

    let topic = body.topic.clone();
    let key_id = claims.key_id;
    let target_data = body.target_data.clone();

    delete_subscription(topic, Some(key_id), target_data).await?;

    Ok(HttpResponse::Ok().finish())
}
