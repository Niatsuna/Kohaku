use serde::{self, Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::utils::{comm::ws::send_message, error::KohakuError};

pub mod auth;
pub mod ws;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum MessageType {
    #[serde(rename = "auth")]
    Authorization,
    #[serde(rename = "ping")]
    Ping { id: String },
    #[serde(rename = "pong")]
    Pong { id: String },
    #[serde(rename = "notification")]
    Notification { data: serde_json::Value },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub timestamp: i64,
    pub message_id: String,
    pub message: MessageType,
}

pub async fn process_message(data: serde_json::Value) -> Result<(), KohakuError> {
    // TODO: Implement actual behaivor based on features
    let datas = data.as_str().unwrap();
    info!(datas);
    Ok(())
}

/// Notifies client with given payload.
/// Requirement: Payload must be serializeable
pub async fn notify_client<T: Serialize>(payload: T) -> Result<(), KohakuError> {
    let message = MessageType::Notification {
        data: json!(payload),
    };
    send_message(message).await.map_err(|e| {
        KohakuError::InternalServerError(format!("Couldn't notify client via websocket: {e}"))
    })
}
