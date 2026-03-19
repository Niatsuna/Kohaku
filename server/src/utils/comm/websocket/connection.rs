use std::time::Duration;

use actix_ws::{Closed, Message, MessageStream, Session};
use serde::Serialize;
use tokio::sync::{
    broadcast,
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tracing::{error, info};

use crate::utils::error::KohakuError;

const HEARTBEAT_INTERVAL_SEC: u64 = 30;
const HEARTBEAT_MAX_MISSED: i32 = 3;

pub struct WsConnectionHandle {
    pub key_id: i32,
    outgoing_w: UnboundedSender<Message>,
    pub shutdown_w: broadcast::Sender<()>,
    pub shutdown_r: broadcast::Receiver<()>,
}

impl WsConnectionHandle {
    pub fn send<T: Serialize>(&self, payload: &T) -> Result<(), KohakuError> {
        let content = serde_json::to_string(payload).map_err(|e| {
            KohakuError::ValidationError(format!("Failed to serialize payload: {}", e))
        })?;
        let message = Message::Text(content.into());
        self.outgoing_w.send(message).map_err(|e| {
            KohakuError::WebsocketError(format!("Failed to queue outgoing message: {}", e))
        })
    }

    pub fn disconnect(&self) -> Result<(), KohakuError> {
        info!(
            "[WS - Connection] Closing connection with associated key id '{}'",
            self.key_id
        );
        match self.shutdown_w.send(()) {
            Ok(_) => Ok(()),
            Err(e) => Err(KohakuError::WebsocketError(format!(
                "Failed to queue shutdown message: {}",
                e
            ))),
        }
    }
}

pub struct WsConnection {
    pub key_id: i32,
    session: Session,
    incoming_r: MessageStream,

    outgoing_w: UnboundedSender<Message>,
    outgoing_r: UnboundedReceiver<Message>,

    incoming_w: UnboundedSender<Message>,
    incoming_rx: UnboundedReceiver<Message>,

    heartbeat_w: UnboundedSender<()>,
    heartbeat_r: UnboundedReceiver<()>,

    shutdown_w: broadcast::Sender<()>,
    shutdown_r: broadcast::Receiver<()>,
}

impl WsConnection {
    pub fn new(key_id: i32, session: Session, stream: MessageStream) -> Self {
        let (shutdown_w, shutdown_r) = broadcast::channel(1);
        let (outgoing_w, outgoing_r) = unbounded_channel();
        let (heartbeat_w, heartbeat_r) = unbounded_channel();
        let (incoming_w, incoming_rx) = unbounded_channel();

        Self {
            key_id,
            session,
            incoming_r: stream,
            outgoing_w,
            outgoing_r,
            incoming_w,
            incoming_rx,
            heartbeat_w,
            heartbeat_r,
            shutdown_w,
            shutdown_r,
        }
    }

    pub fn get_handle(&self) -> WsConnectionHandle {
        WsConnectionHandle {
            key_id: self.key_id.clone(),
            outgoing_w: self.outgoing_w.clone(),
            shutdown_w: self.shutdown_w.clone(),
            shutdown_r: self.shutdown_r.resubscribe(),
        }
    }

    pub async fn run(self) {
        let key_id = self.key_id;
        let shutdown_w = self.shutdown_w.clone();

        // Spawn outgoing message handler
        let session_out = self.session.clone();
        let mut outgoing_r = self.outgoing_r;
        let shutdown_w_out = shutdown_w.clone();
        let mut shutdown_r_out = self.shutdown_r.resubscribe();

        let outgoing_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_r_out.recv() => {
                        info!("[WS - Connection] Shutdown received for '{}'. Closing internal channel.", key_id);
                        outgoing_r.close();
                    }

                    Some(msg) = outgoing_r.recv() => {
                        if let Err(e) = Self::handle_outgoing(session_out.clone(), msg).await {
                            info!("[WS - Connection] Failed to send, closing for '{}' - Error : {}", key_id, e);
                            if let Err(e) = shutdown_w_out.send(()) {
                                error!("[WS - Connection] Failed to queue shutdown message: {}", e);
                            }
                            break;
                        }
                    }

                    else => {
                        info!("[WS - Connection] Internal channel closed for {}", key_id);
                        break;
                    }
                }
            }
        });

        // Spawn heartbeat handler
        let mut session_htbt = self.session.clone();
        let mut htbt_r = self.heartbeat_r;
        let shutdown_w_htbt = shutdown_w.clone();
        let mut shutdown_r_htbt = self.shutdown_r.resubscribe();
        let htbt_handle = tokio::spawn(async move {
            let mut missed_pings = 0;
            let htbt_interval = Duration::from_secs(HEARTBEAT_INTERVAL_SEC);
            loop {
                tokio::select! {
                    _ = shutdown_r_htbt.recv() => {
                        info!("[WS - Connection] Shutdown received for '{}'. Closing heartbeat channel.", key_id);
                        htbt_r.close();
                    }

                    _ = tokio::time::sleep(htbt_interval) => {
                        if missed_pings >= HEARTBEAT_MAX_MISSED {
                            info!("[WS - Connection] Client missed too many heartbeats, starting disconnect for '{}'", key_id);
                            if let Err(e) = shutdown_w_htbt.send(()) {
                                error!("[WS - Connection] Failed to queue shutdown message: {}", e);
                            }
                            break;
                        }

                        missed_pings += 1;
                        if let Err(e) = session_htbt.ping(b"").await {
                            error!("[WS - Connection] Failed to send ping: {}", e);
                            if let Err(e) = shutdown_w_htbt.send(()) {
                                error!("[WS - Connection] Failed to queue shutdown message: {}", e);
                            }
                            break;
                        }
                    }

                    Some(_) = htbt_r.recv() => {
                        missed_pings = 0;
                    }

                    else => {
                        info!("[WS - Connection] Heartbeat channel closed for {}", key_id);
                        break;
                    }
                }
            }
        });

        // Spawn incoming message handler
        let session_in = self.session.clone();
        let mut incoming_r = self.incoming_r;
        let htbt_w_in = self.heartbeat_w.clone();
        let shutdown_w_in = shutdown_w.clone();
        let mut shutdown_r_in = self.shutdown_r.resubscribe();
        actix_web::rt::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_r_in.recv() => {
                        info!("[WS - Connection] Shutdown received for '{}'. Closing external channel.", key_id);
                        break;
                    }

                    Some(Ok(msg)) = incoming_r.recv() => {
                        if !Self::handle_incoming(session_in.clone(), htbt_w_in.clone(), msg).await {
                            info!("[WS - Connection] Client requested close for '{}'.", key_id);
                            if let Err(e) = shutdown_w_in.send(()) {
                                error!("[WS - Connection] Failed to queue shutdown message: {}", e);
                            }
                            break;
                        }
                    }

                    else => {
                        info!("[WS - Connection] External channel closed for {}", key_id);
                        break;
                    }
                }
            }
            let _ = tokio::join!(outgoing_handle, htbt_handle);
            let _ = self.session.close(None).await;
            info!(
                "[WS - Connection] Client connection to '{}' closed entirely.",
                key_id
            );
        });
    }

    async fn handle_outgoing(mut session: Session, message: Message) -> Result<(), KohakuError> {
        match message {
            Message::Text(text) => session.text(text).await,
            Message::Binary(bin) => session.binary(bin).await,
            Message::Ping(bytes) => session.ping(&bytes).await,
            Message::Pong(bytes) => session.pong(&bytes).await,
            Message::Close(reason) => session.close(reason).await,
            _ => Ok(()),
        }
        .map_err(|e| KohakuError::WebsocketError(format!("Failed to send message: {}", e)))
    }

    async fn handle_incoming(
        mut session: Session,
        heartbeat_w: UnboundedSender<()>,
        message: Message,
    ) -> bool {
        let res = match message {
            Message::Close(_) => Err(Closed),
            Message::Ping(bytes) => session.pong(&bytes).await,
            Message::Pong(_) => {
                let temp = heartbeat_w.send(());
                if let Err(e) = temp {
                    error!("[WS - Connection] Failed to queue heartbeat message: {}", e);
                    Err(Closed)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        };

        res.is_ok()
    }
}
