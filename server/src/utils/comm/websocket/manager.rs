use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use actix_ws::{Message, MessageStream, Session};
use serde::Serialize;
use tokio::sync::{mpsc::UnboundedSender, OnceCell};
use tracing::{error, info};

use crate::utils::{
    comm::websocket::connection::{WsClientInfo, WsConnection},
    error::KohakuError,
};

static WS_CONNECTION_MANAGER: OnceCell<Arc<WsConnectionManager>> = OnceCell::const_new();

pub struct WsConnectionManager {
    connections: RwLock<HashMap<i32, UnboundedSender<Message>>>,
}

impl WsConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Prepares the necessary connection and registers it inside the manager.
    /// If a connection via this API key is already present, no new connection will be established.
    ///
    /// # Parameters
    /// - `info` : Necessary information about the connected client
    /// - `session` : Current active session to the client
    /// - `stream` : Current active stream from the client
    ///
    /// # Returns
    /// A [`Option<WsConnection>`] which is either:
    /// - [`Some`] : A [`WsConnection`] that is registered inside the manager and can be executed via [`WsConnection::run`]
    /// - [`None`] : If the API key is already in use with some connection
    pub async fn add_connection(
        &self,
        info: WsClientInfo,
        session: Session,
        stream: MessageStream,
    ) -> Option<WsConnection> {
        let key_id = info.key_id;
        if self.connections.read().unwrap().contains_key(&key_id) {
            return None;
        }
        let conn = WsConnection::new(info, session, stream);
        let sender = conn.server_tx.clone();
        self.connections.write().unwrap().insert(key_id, sender);
        Some(conn)
    }

    /// Removes a connection from the manager, making it unable to receive messages from the server
    ///
    /// # Parameters
    /// - `key_id` - API key identifier for connections in the manager
    pub async fn remove_connection(&self, key_id: &i32) {
        self.connections.write().unwrap().remove(key_id);
    }

    /// Sends a [`Serialize`]-able payload to multiple clients.
    ///
    /// # Parameters
    /// - `payload` - Generic serializable content
    /// - `key_ids` - Vector of API key ids as targets. If [`None`] the payload will be send to all active connections
    ///
    /// # Type Parameters
    /// - `T` - Any struct that derives [`Serialize`]
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] - Indicating that the queueing of the message was successful
    /// - [`Err`] - A [`KohakuError`] indicating that ANY operation failed
    pub async fn broadcast<T: Serialize>(
        &self,
        payload: T,
        key_ids: Option<Vec<i32>>,
    ) -> Result<(), KohakuError> {
        let collections = match key_ids {
            Some(given) => given,
            None => {
                let stored = self.connections.read().unwrap().clone();
                stored.keys().copied().collect::<Vec<i32>>()
            }
        };
        let mut successful = 0;
        let mut failed_clients = Vec::new();

        for key_id in collections {
            match self.send_to_client(&payload, &key_id).await {
                Ok(_) => successful += 1,
                Err(e) => {
                    error!("[WS - Broadcast] {}", e);
                    failed_clients.push(key_id)
                }
            }
        }

        // Clean up
        for key_id in &failed_clients {
            self.remove_connection(key_id).await;
        }
        info!(
            "[WS - Broadcast] Broadcasted 1 message successfully {} time(s) and failed {} time(s)",
            successful,
            &failed_clients.len()
        );
        Ok(())
    }

    /// Sends a [`Serialize`]-able payload to a connected client.
    ///
    /// # Parameters
    /// - `payload` - Generic serializable content
    /// - `key_id` - Identifier for target client via API key id
    ///
    /// # Type Parameters
    /// - `T` - Any struct that derives [`Serialize`]
    ///
    /// # Returns
    /// A [`Result`] which is either
    /// - [`Ok`] - Indicating that the queueing of the message was successful
    /// - [`Err`] - A [`KohakuError`] indicating that ANY operation failed
    pub async fn send_to_client<T: Serialize>(
        &self,
        payload: T,
        key_id: &i32,
    ) -> Result<(), KohakuError> {
        let connections = self.connections.read().unwrap().clone();
        let content = serde_json::to_string(&payload).unwrap();

        if let Some(sender) = connections.get(key_id) {
            sender.send(Message::Text(content.into())).map_err(|e| {
                KohakuError::WebsocketError(format!(
                    "Failed to send to client with key_id {} : {}",
                    key_id, e
                ))
            })
        } else {
            Err(KohakuError::ExternalServiceError(format!(
                "Client with key id {} not found",
                key_id
            )))
        }
    }
}

/// Initializes a globally unqiue and accessible [`WsConnectionManager`] instance.
///
/// # Parameters
/// - `encryption_key` : A [`String`]-based key for JWT encryption. Can be found in the configuration and is loaded as a env
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : [`WsConnectionManager`] is now accessible via [get_manager]
/// - [`Err`] : A [KohakuError::InternalServerError] if the [`manager`] is already initialized
pub fn init_manager() -> Result<(), KohakuError> {
    let service = Arc::new(WsConnectionManager::new());
    WS_CONNECTION_MANAGER.set(service).map_err(|_| {
        KohakuError::WebsocketError("Websocket Connection Manager already initialized".to_string())
    })?;
    Ok(())
}

/// Get current [`WsConnectionManager`] instance.
///
/// # Returns
/// A [`Result`] which is either
/// - [`Ok`] : A [`Arc<WsConnectionManager>`] to gain access to the functionalities of the [`WsConnectionManager`]
/// - [`Err`] : A [KohakuError::InternalServerError] if the [`WsConnectionManager`] was not prior initialized via [`init_manager`]
pub fn get_manager() -> Result<Arc<WsConnectionManager>, KohakuError> {
    let service = WS_CONNECTION_MANAGER.get();
    if service.is_none() {
        return Err(KohakuError::WebsocketError(
            "Websocket Connection Manager not initialized - call init_manager first!".to_string(),
        ));
    }
    Ok(service.unwrap().clone())
}
