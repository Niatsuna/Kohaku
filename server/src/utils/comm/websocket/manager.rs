use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use actix_ws::{MessageStream, Session};
use serde::Serialize;
use tokio::sync::OnceCell;
use tracing::info;

use crate::utils::{
    comm::websocket::connection::{WsConnection, WsConnectionHandle},
    error::KohakuError,
};

static WS_CONNECTION_MANAGER: OnceCell<Arc<WsConnectionManager>> = OnceCell::const_new();

pub struct WsConnectionManager {
    connections: Arc<RwLock<HashMap<i32, WsConnectionHandle>>>,
}

impl WsConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Checks if any connection with said uuid or key is currently active
    pub fn check_if_active(&self, key_id: &i32) -> bool {
        self.connections.read().unwrap().contains_key(key_id)
    }

    pub async fn register(
        &self,
        key_id: i32,
        session: Session,
        stream: MessageStream,
    ) -> Result<WsConnection, KohakuError> {
        let connection = WsConnection::new(key_id, session, stream);

        let handle = connection.get_handle();
        let mut shutdown_cleanup = handle.shutdown_r.resubscribe();

        let mut conns = self.connections.write().map_err(|e| {
            KohakuError::WebsocketError(format!(
                "Failed to gain access to connection hashmap: {}",
                e
            ))
        })?;
        conns.insert(key_id, handle);

        let connections = self.connections.clone();
        tokio::spawn(async move {
            let _ = shutdown_cleanup.recv().await;
            info!(
                "[WS - Manager] Shutdown detected for '{}', cleaning up",
                key_id
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut conns = connections.write().unwrap();
            conns.remove(&key_id);
        });

        Ok(connection)
    }

    pub async fn disconnect(&self, key_id: &i32) {
        let connections = self.connections.read().unwrap();
        let handle = connections.get(key_id);
        if let Some(h) = handle {
            let _ = h.disconnect();
        }
    }

    pub async fn send_to_client<T: Serialize>(
        &self,
        key_id: &i32,
        payload: &T,
    ) -> Result<(), KohakuError> {
        let connections = self.connections.read().unwrap();
        if let Some(conn) = connections.get(key_id) {
            conn.send(payload)
        } else {
            Err(KohakuError::WebsocketError("Not connected".to_string()))
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
