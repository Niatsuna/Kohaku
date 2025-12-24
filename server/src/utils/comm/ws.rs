use std::{sync::Arc, time::Duration};

use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use chrono::Utc;
use futures_util::StreamExt as _;
use once_cell::sync::OnceCell;
use tokio::{
    sync::{Mutex, RwLock},
    time::interval,
};
use tracing::{error, info};

use crate::utils::{
    comm::{
        auth::{sign_message, verify_message},
        process_message, MessageType, WsMessage,
    },
    config::get_config,
    error::KohakuError,
};

/// Shared Session
static CLIENT_SESSION: OnceCell<Arc<RwLock<Option<ClientConnection>>>> = OnceCell::new();

/// Complete client connection including session and state
struct ClientConnection {
    session: Session,
    authenticated: bool,
}

/// RateLimiter for WebSocket messages.
pub struct RateLimiter {
    messages: Vec<i64>,
    max_messages: usize,
    window_secs: i64,
}

impl RateLimiter {
    pub fn new(max_messages: usize, window_secs: i64) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            window_secs,
        }
    }

    pub fn check_and_add(&mut self) -> bool {
        let now = Utc::now().timestamp();
        let cutoff = now - self.window_secs;

        // Remove old messages
        self.messages.retain(|&t| t > cutoff);

        if self.messages.len() >= self.max_messages {
            return false;
        }
        self.messages.push(now);
        true
    }
}

pub fn init_client_session() {
    CLIENT_SESSION.get_or_init(|| Arc::new(RwLock::new(None)));
}

/// Sends a message to the connected client.
pub async fn send_message(input: MessageType) -> Result<(), KohakuError> {
    let config = get_config();

    let session_lock = CLIENT_SESSION
        .get()
        .ok_or(KohakuError::InternalServerError(
            "[WS] WebSocket Client not initialized".to_string(),
        ))?;

    let mut session_guard = session_lock.write().await;

    let message = WsMessage {
        timestamp: Utc::now().timestamp(),
        message_id: uuid::Uuid::new_v4().to_string(),
        message: input,
    };

    if let Some(client) = session_guard.as_mut() {
        let signed = sign_message(&message, &config.secret);
        client
            .session
            .text(signed)
            .await
            .map_err(|e| KohakuError::OperationError {
                operation: "Websocket-Session-Text".to_string(),
                source: Box::new(e),
            })?;
        Ok(())
    } else {
        Err(KohakuError::InternalServerError(
            "[WS] No client connected".to_string(),
        ))
    }
}

/// Close current connection
async fn close_session() {
    if let Some(session_lock) = CLIENT_SESSION.get() {
        let mut session_guard = session_lock.write().await;
        if let Some(client) = session_guard.take() {
            let _ = client.session.close(None).await;
            info!("[WS] Session closed");
        }
    }
}

/// Handles current connection on the websocket
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, KohakuError> {
    let config = get_config();
    let secret = config.secret.clone();

    let (response, session, mut stream) = actix_ws::handle(&req, stream).map_err(|e| {
        KohakuError::InternalServerError(format!("[WS] Error while handling incoming stream: {e}"))
    })?;

    // Store the session
    {
        let session_lock = CLIENT_SESSION
            .get()
            .ok_or(KohakuError::InternalServerError(
                "[WS] Client session not initialized".to_string(),
            ))?;
        let mut session_guard = session_lock.write().await;

        // Close any existing connection
        if let Some(old_client) = session_guard.take() {
            info!("[WS] Closing existing connection");
            let _ = old_client.session.close(None).await;
        }

        // Store new connection
        *session_guard = Some(ClientConnection {
            session,
            authenticated: false,
        });
        info!("[WS] New client session stored");
    }

    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(20, 60)));

    // Heartbeat Task - runs independently
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let ping_msg = MessageType::Ping {
                id: uuid::Uuid::new_v4().to_string(),
            };

            if let Err(e) = send_message(ping_msg).await {
                error!("[WS] Failed to send heartbeat: {}", e);
                break;
            }
        }
        info!("[WS] Heartbeat task stopped");
    });

    // Reader Task: Handle incoming messages in actix-rt context (not tokio::spawn)
    // This allows us to use the non-Send stream
    actix_rt::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => {
                    // Rate limiting
                    {
                        let mut limiter = rate_limiter.lock().await;
                        if !limiter.check_and_add() {
                            error!("[WS] - Rate limit exceeded");
                            close_session().await;
                            break;
                        }
                    }

                    // Verify and parse message
                    match verify_message(&text, &secret) {
                        Ok(message) => {
                            info!("[WS] - Received valid message: {:?}", message.message);

                            match message.message {
                                MessageType::Authorization => {
                                    // Set connection to authenticated
                                    let session_lock = CLIENT_SESSION.get().unwrap();
                                    let mut session_guard = session_lock.write().await;
                                    if let Some(client) = session_guard.as_mut() {
                                        client.authenticated = true;
                                        info!("[WS] - Client authenticated!");
                                    }
                                }
                                MessageType::Pong { id } => {
                                    info!("[WS] - Received pong: {}", id);
                                }
                                MessageType::Notification { data } => {
                                    // Check authentication
                                    let is_authenticated = {
                                        let session_lock = CLIENT_SESSION.get().unwrap();
                                        let session_guard = session_lock.read().await;
                                        session_guard
                                            .as_ref()
                                            .map(|c| c.authenticated)
                                            .unwrap_or(false)
                                    };

                                    if !is_authenticated {
                                        error!(
                                            "[WS] - Received message from unauthenticated client"
                                        );
                                        close_session().await;
                                        break;
                                    }

                                    // Process message
                                    if let Err(e) = process_message(data).await {
                                        error!("[WS] - Error processing message: {}", e);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            error!("[WS] - Invalid message: {}", e);
                        }
                    }
                }
                Message::Ping(bytes) => {
                    info!("[WS] - Received ping, sending pong");
                    // Send pong through
                    let session_lock = CLIENT_SESSION.get().unwrap();
                    let mut session_guard = session_lock.write().await;
                    if let Some(client) = session_guard.as_mut() {
                        let _ = client.session.pong(&bytes).await;
                    }
                }
                Message::Close(_) => {
                    info!("[WS] - Client disconnect!");
                    close_session().await;
                    break;
                }
                _ => {}
            }
        }

        // Clean up on exit
        close_session().await;
        info!("[WS] Reader task stopped");
    });

    Ok(response)
}
