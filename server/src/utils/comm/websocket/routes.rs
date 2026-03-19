use actix_web::{web, HttpRequest, HttpResponse};
use tracing::info;

use crate::utils::{
    comm::{auth::check_authorization_token, websocket::manager::get_manager},
    error::KohakuError,
};

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, KohakuError> {
    let claims = check_authorization_token(&req, Some(vec!["events:subscribe"])).await?;
    let owner = claims.owner;
    let key_id = claims.key_id;

    let manager = get_manager()?;
    if manager.check_if_active(&key_id) {
        return Err(KohakuError::WebsocketError(
            "API Key already in use".to_string(),
        ));
    }
    let (response, session, msg_stream) =
        actix_ws::handle(&req, stream).map_err(|e| KohakuError::WebsocketError(e.to_string()))?;
    if let Ok(conn) = manager.register(key_id, session, msg_stream).await {
        info!(
            "[WS] Established connection for key with owner '{}' (id: {})",
            owner, key_id
        );
        conn.run().await;
    } else {
        return Err(KohakuError::WebsocketError(
            "Couldn't create WebSocketConnection!".to_string(),
        ));
    }
    Ok(response)
}
