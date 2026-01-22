use std::str::FromStr;

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::utils::{
    comm::{
        auth::{check_authorization_key, extract_key},
        websocket::{connection::WsClientInfo, manager::get_manager},
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
    if manager.check_connection_by_key_id(&verified_key.id)
        || manager.check_connection_by_uuid(&client_id)
    {
        return Err(KohakuError::Conflict(
            "API Key or client UUID already in use!".to_string(),
        ));
    }

    let info = WsClientInfo {
        client_id,
        owner: verified_key.owner,
        key_id: verified_key.id,
    };

    let (response, session, msg_stream) =
        actix_ws::handle(&req, stream).map_err(|e| KohakuError::WebsocketError(e.to_string()))?;

    let conn = manager
        .add_connection(info.clone(), session, msg_stream)
        .await;
    if let Some(conn_) = conn {
        info!(
            "[WS - Conn] Established new connection {} for key with id {}",
            info.client_id, verified_key.id
        );

        let payload = json!({ "Bearer" : client_id });
        let content = serde_json::to_string(&payload).unwrap();
        let _ = conn_.server_tx.send(Message::Text(content.into()));

        conn_.run(manager);
    } else {
        return Err(KohakuError::WebsocketError(
            "Couldn't create WebSocketConnection!".to_string(),
        ));
    }
    Ok(response)
}
