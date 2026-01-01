use std::{sync::Arc, time::Duration};

use actix_ws::{Message, MessageStream, Session};
use futures_util::StreamExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::info;
use uuid::Uuid;

use crate::utils::comm::websocket::manager::WsConnectionManager;

const HEARTBEAT_INTERVAL_SEC: u64 = 30;
const HEARTBEAT_MAX_MISSED: i32 = 3;

#[derive(Debug, Clone)]
pub struct WsClientInfo {
    pub client_id: Uuid,
    pub owner: String,
    pub key_id: i32,
}

pub struct WsConnection {
    pub info: WsClientInfo,
    session: Session,
    extern_rx: MessageStream,
    pub server_tx: UnboundedSender<Message>,
    server_rx: UnboundedReceiver<Message>,
    heartbeat_tx: UnboundedSender<()>,
    pub heartbeat_rx: UnboundedReceiver<()>,
}

impl WsConnection {
    pub fn new(info: WsClientInfo, session: Session, stream: MessageStream) -> Self {
        let (server_tx, server_rx) = unbounded_channel::<Message>();
        let (heartbeat_tx, heartbeat_rx) = unbounded_channel::<()>();

        WsConnection {
            info,
            session,
            extern_rx: stream,
            server_tx,
            server_rx,
            heartbeat_tx,
            heartbeat_rx,
        }
    }

    /// Start WebSocket Connection : Spawn all three tasks to have a functioning connection
    ///
    /// Tasks:
    /// - [`WsConnection::send`] - Sends queued messages from the server to the client
    /// - [`WsConnection::heartbeat`] - Checks if the client is still alive, if not close connection
    /// - [`WsConnection::receive`] - Handles incoming messages from the client and propagates pongs (Heartbeats) to the heartbeat task
    ///
    /// # Parameters
    /// - `manager` : The associated [`WsConnectionManager`]. Will be used to remove this connection when its closes
    pub fn run(self, manager: Arc<WsConnectionManager>) {
        let client_id = self.info.client_id;
        let key_id = self.info.key_id;
        let session = self.session;
        let extern_rx = self.extern_rx;
        let server_rx = self.server_rx;
        let heartbeat_tx = self.heartbeat_tx;
        let heartbeat_rx = self.heartbeat_rx;

        let session_send = session.clone();
        let send_handle = tokio::spawn(async move {
            Self::send(session_send, server_rx).await;
        });

        let session_htbt = session.clone();
        let htbt_handle = tokio::spawn(async move {
            Self::heartbeat(session_htbt, heartbeat_rx, client_id, key_id).await;
        });

        let session_recv = session.clone();

        actix_web::rt::spawn(async move {
            Self::receive(session_recv, extern_rx, heartbeat_tx).await;

            // Wait for the other tasks to complete
            let _ = tokio::join!(send_handle, htbt_handle);
            info!("[WS - Conn] Client {} connection ended, closing session and removing from manager [Key: {}]", client_id, key_id);

            let _ = session.close(None).await;
            manager.remove_connection(&key_id).await;
        });
    }

    /// Sends queued data from the server to the connected client.
    /// Will stop if any message cannot reach the client.
    ///
    /// # Parameters
    /// - `session` : The connected associated [`Session`] to the client
    /// - `server_rx`: Receiver half of the internal channel. Incoming messages are messages from other services within the server
    async fn send(session: Session, mut server_rx: UnboundedReceiver<Message>) {
        while let Some(msg) = server_rx.recv().await {
            let mut session = session.clone();
            let result = match msg {
                Message::Text(text) => session.text(text).await,
                Message::Binary(bin) => session.binary(bin).await,
                Message::Ping(bytes) => session.ping(&bytes).await,
                Message::Pong(bytes) => session.pong(&bytes).await,
                Message::Close(reason) => session.close(reason).await,
                _ => Ok(()),
            };

            if result.is_err() {
                break;
            }
        }
    }

    /// Receives externally messages from the client that reached the server
    /// Will only react to `Ping`, `Pong` and `Close` messages and will stop if either a closing event was detected
    /// or the resulting pong does not reach the client.
    ///
    /// # Parameters
    /// - `session` : The connected associated [`Session`] to the client
    /// - `server_rx`: Receiver half of the internal channel. Incoming messages are messages from other services within the server
    /// - `heartbeat_tx` : Sender half of the internal heartbeat channel. Incoming pongs will be propagated to this channel to reset the missed pings counter
    async fn receive(
        mut session: Session,
        mut extern_rx: MessageStream,
        heartbeat_tx: UnboundedSender<()>,
    ) {
        while let Some(Ok(msg)) = extern_rx.next().await {
            match msg {
                Message::Close(_) => {
                    info!("[WS - Conn] Client send closing event, disconnecting");
                    let _ = session.close(None).await;
                    return;
                }
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Pong(_) => {
                    let _ = heartbeat_tx.send(());
                }
                _ => {}
            }
        }
    }

    /// Handles server-sided heartbeats to check if the connected client is still responding.
    ///
    /// Sends in `HEARTBEAT_INTERVAL_SEC` intervals a `ping` at the connected client.
    /// `Pong`s reset the counter for missed pings.
    /// Discard connection if the missed pings are reaching the threshold `HEARTBEAT_MAX_MISSED`
    ///
    /// # Parameters
    /// - `session` : The connected associated [`Session`] to the client
    /// - `heartbeat_rx` : Receiver half of the internal heartbeat channel. Incoming pongs will be propagated to this channel to reset the missed pings counter
    /// - `client_id` : Readable identifier of connection (logging purposes)
    /// - `key_id` : Readable identifier of API key associated with the connected client (logging purposes)
    async fn heartbeat(
        mut session: Session,
        mut heartbeat_rx: UnboundedReceiver<()>,
        client_id: Uuid,
        key_id: i32,
    ) {
        let mut missing_pings = 0;
        let heartbeat_interval = Duration::from_secs(HEARTBEAT_INTERVAL_SEC);

        loop {
            tokio::select! {
              _ = tokio::time::sleep(heartbeat_interval) => {
                if missing_pings >= HEARTBEAT_MAX_MISSED {
                  info!("[WS - Conn] Client {} missed too many heartbeats, disconnecting [Key {}]", client_id, key_id);
                  let _ = session.close(None).await;
                  break;
                }

                // New pings
                missing_pings += 1;
                if session.ping(b"").await.is_err() {
                  break;
                }
              }

              // Reset missing pings
              Some(_) = heartbeat_rx.recv() => {
                missing_pings = 0;
              }
            }
        }
    }
}
