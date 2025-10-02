use std::{sync::Arc, time::Duration};

use actix_web::{web, Error, HttpRequest, HttpResponse};
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
};

/*
  WebSockert (WS) module for bidirectional Client-Server communication.
  This is NOT for the general "information" but rather for data flows that require a level of authentication.
  For example: If a command in discord should update an entry in the database.
  For general data flow, like e.g. "getting the list of characters for a game", please look at the REST API handlers!
*/

/// Shared Session
static CLIENT_SESSION: OnceCell<Arc<RwLock<Option<Session>>>> = OnceCell::new();

/// RateLimiter for WebSocket messages.
struct RateLimiter {
    messages: Vec<i64>,
    max_messages: usize,
    window_secs: i64,
}

impl RateLimiter {
    fn new(max_messages: usize, window_secs: i64) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            window_secs,
        }
    }

    fn check_and_add(&mut self) -> bool {
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

/// Current state of the connection (1:1 Connection)
struct ConnectionState {
    rate_limiter: RateLimiter,
    authenticated: bool,
}

pub fn init_client_session() {
    CLIENT_SESSION.get_or_init(|| Arc::new(RwLock::new(None)));
}

/// Sends a message to the connected client.
pub async fn send_message(input: MessageType) -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config();

    let session_lock = CLIENT_SESSION
        .get()
        .ok_or("Client session not initialized")?;
    let mut session_guard = session_lock.write().await;

    let message = WsMessage {
        timestamp: Utc::now().timestamp(),
        message_id: uuid::Uuid::new_v4().to_string(),
        message: input,
    };

    if let Some(session) = session_guard.as_mut() {
        let signed = sign_message(&message, &config.secret);
        session.text(signed).await?;
        Ok(())
    } else {
        Err("No client connected".into())
    }
}

/// Close current connection
async fn close_session() {
    let session_lock = CLIENT_SESSION
        .get()
        .ok_or("Client session not initialized")
        .unwrap();
    let mut session_guard = session_lock.write().await;
    if let Some(session) = session_guard.as_mut() {
        let sess = session.clone();
        let _ = sess.close(None).await;
    }
}

/// Handles current connection on the websocket
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let config = get_config();
    let secret = config.secret.clone();

    let (response, mut session, stream) = actix_ws::handle(&req, stream)?;
    let state = Arc::new(Mutex::new(ConnectionState {
        rate_limiter: RateLimiter::new(20, 60),
        authenticated: false,
    }));

    // Heartbeat Task
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let ping_msg = MessageType::Ping {
                id: uuid::Uuid::new_v4().to_string(),
            };

            if send_message(ping_msg).await.is_err() {
                break;
            }
        }
    });

    // Reader Task : Handling incoming messages
    actix_rt::spawn(async move {
        let mut stream = stream;

        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => {
                    // General incoming message
                    let mut guard = state.lock().await;
                    // Rate limiting
                    if !guard.rate_limiter.check_and_add() {
                        error!("[WS] - Rate limit exceeded");
                        close_session().await;
                        break;
                    }

                    // Verify and parse message
                    match verify_message(&text, &secret) {
                        Ok(message) => {
                            info!("[WS] - Received valid message: {:?}", message.message);
                            match message.message {
                                MessageType::Authorization => {
                                    // Valid signature -> Set connection to be authenticated
                                    guard.authenticated = true;
                                    info!("[WS] - Client authenticated!");
                                }
                                MessageType::Pong { id } => {
                                    info!("[WS] - Received pong: {}", id);
                                }
                                _ => {
                                    if !guard.authenticated {
                                        error!(
                                            "[WS] - Received message from unauthenticated client"
                                        );
                                        close_session().await;
                                        break;
                                    }
                                    // Process message
                                    if let MessageType::Notification { data } = message.message {
                                        // TODO: Catch errors
                                        let _ = process_message(data).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("[WS] - Invalid message: {}", e);
                        }
                    }
                }
                Message::Ping(bytes) => {
                    // Ping message
                    info!("[WS] - Received ping, sending pong");
                    let _ = state.lock().await;
                    let _ = session.pong(&bytes).await;
                }
                Message::Close(_) => {
                    // Disconnect
                    info!("[WS] - Client disconnect!");
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(response)
}
