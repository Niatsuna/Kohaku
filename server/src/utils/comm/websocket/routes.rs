use std::str::FromStr;

use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::utils::{
    comm::{
        auth::{check_authorization_key, extract_key},
        websocket::manager::get_manager,
    },
    error::KohakuError,
};

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, KohakuError> {
    let api_key = extract_key(&req);
    if api_key.is_none() {
        return Err(KohakuError::Unauthorized(
            "Missing API key header".to_string(),
        ));
    }
    let verified_key = check_authorization_key(api_key.unwrap()).await?;
    let client_id = req.headers().get("UUID").and_then(|h| h.to_str().ok());

    let client_id = match client_id {
        Some(cid) => Uuid::from_str(cid)
            .map_err(|_| KohakuError::BadRequest("Given UUID is malformed!".to_string()))?,
        None => Uuid::new_v4(),
    };

    let manager = get_manager()?;
    if manager.check_if_active(Some(client_id), Some(verified_key.id)) {
        return Err(KohakuError::WebsocketError(
            "UUID and/or API Key alread in use".to_string(),
        ));
    }

    let (response, session, msg_stream) =
        actix_ws::handle(&req, stream).map_err(|e| KohakuError::WebsocketError(e.to_string()))?;

    if let Ok(conn) = manager
        .register(client_id, verified_key.id, session, msg_stream)
        .await
    {
        info!("[WS] Established connection to '{}'", client_id);
        let payload = json!({ "Bearer" : client_id });
        let _ = manager.send_to_client(&client_id, &payload).await;
        conn.run().await;
    } else {
        return Err(KohakuError::WebsocketError(
            "Couldn't create WebSocketConnection!".to_string(),
        ));
    }
    Ok(response)
}
