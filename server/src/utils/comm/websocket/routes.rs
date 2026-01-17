use actix_web::{web, HttpRequest, HttpResponse};
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

    let info = WsClientInfo {
        client_id: Uuid::new_v4(),
        owner: verified_key.owner,
        key_id: verified_key.id,
    };

    let (response, session, msg_stream) =
        actix_ws::handle(&req, stream).map_err(|e| KohakuError::WebsocketError(e.to_string()))?;

    let manager = get_manager()?;
    let conn = manager
        .add_connection(info.clone(), session, msg_stream)
        .await;
    if let Some(conn_) = conn {
        info!(
            "[WS - Conn] Established new connection {} for key with id {}",
            info.client_id, verified_key.id
        );
        conn_.run(manager);
    } else {
        return Err(KohakuError::WebsocketError(
            "Couldn't create WebSocketConnection!".to_string(),
        ));
    }
    Ok(response)
}
