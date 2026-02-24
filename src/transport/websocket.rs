use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{SinkExt, StreamExt};
use serde_json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::transport::schema::{IncomingMessage, OutgoingMessage, OverlayStatus};

static CONNECTED_CLIENTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub enum WsEvent {
    Message(Box<IncomingMessage>),
    ClientConnected(SocketAddr),
    ClientDisconnected(SocketAddr),
}

pub struct WsConfig {
    pub bind_address: String,
}

pub struct WsServer {
    bind_addr: SocketAddr,
    event_tx: mpsc::UnboundedSender<WsEvent>,
    broadcast_tx: broadcast::Sender<String>,
}

impl WsServer {
    pub fn new(config: WsConfig, event_tx: mpsc::UnboundedSender<WsEvent>) -> Self {
        let addr: SocketAddr = config.bind_address.parse().unwrap_or_else(|_| "127.0.0.1:9001".parse().unwrap());
        let (broadcast_tx, _) = broadcast::channel(256);

        Self {
            bind_addr: addr,
            event_tx,
            broadcast_tx,
        }
    }

    pub fn client_count() -> usize {
        CONNECTED_CLIENTS.load(Ordering::Relaxed)
    }

    pub async fn start(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        println!("[WS] 🌐 WebSocket listening on ws://{}", self.bind_addr);

        let event_tx = self.event_tx;
        let broadcast_tx = self.broadcast_tx;

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let tx = event_tx.clone();
                    let bcast_tx = broadcast_tx.clone();
                    let bcast_rx = broadcast_tx.subscribe();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, addr, tx, bcast_tx, bcast_rx).await {
                            eprintln!("[WS] Client error {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => eprintln!("[WS] Accept error: {}", e),
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    event_tx: mpsc::UnboundedSender<WsEvent>,
    broadcast_tx: broadcast::Sender<String>,
    mut broadcast_rx: broadcast::Receiver<String>,
) -> anyhow::Result<()> {
    let ws_stream = accept_async(stream).await?;
    CONNECTED_CLIENTS.fetch_add(1, Ordering::Relaxed);
    
    let _ = event_tx.send(WsEvent::ClientConnected(addr));
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let welcome = serde_json::to_string(&OutgoingMessage::Status(OverlayStatus {
        active_windows: 0,
        connected_clients: CONNECTED_CLIENTS.load(Ordering::Relaxed),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })).unwrap_or_default();

    let _ = ws_tx.send(Message::Text(welcome)).await;

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                handle_text_message(&text, addr, &event_tx, &broadcast_tx).await;
            }
            Ok(Message::Ping(_)) => {
                let pong = serde_json::to_string(&OutgoingMessage::Pong).unwrap_or_default();
                let _ = broadcast_tx.send(pong);
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    CONNECTED_CLIENTS.fetch_sub(1, Ordering::Relaxed);
    let _ = event_tx.send(WsEvent::ClientDisconnected(addr));

    Ok(())
}

async fn handle_text_message(
    text: &str,
    addr: SocketAddr,
    event_tx: &mpsc::UnboundedSender<WsEvent>,
    broadcast_tx: &broadcast::Sender<String>,
) {
    match IncomingMessage::parse_and_validate(text) {
        Ok(msg) => {
            if let IncomingMessage::ChatMessage(ref payload) = msg {
                let id = payload.get_or_generate_id();
                let ack = serde_json::to_string(&OutgoingMessage::Ack { id }).unwrap_or_default();
                let _ = broadcast_tx.send(ack);
            }

            if let Err(e) = event_tx.send(WsEvent::Message(Box::new(msg))) {
                eprintln!("[WS] Error forwarding message: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[WS] Invalid message from {}: {}", addr, e);
            let error_msg = serde_json::to_string(&OutgoingMessage::Error {
                code: "VALIDATION_ERROR".to_string(),
                message: e.to_string(),
            }).unwrap_or_default();
            let _ = broadcast_tx.send(error_msg);
        }
    }
}
