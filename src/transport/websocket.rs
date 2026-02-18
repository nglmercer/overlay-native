/// Servidor WebSocket para recibir mensajes de plataformas externas
///
/// Escucha conexiones WebSocket en el puerto configurado y retransmite
/// los mensajes validados al sistema de renderizado.
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use serde_json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::transport::schema::{IncomingMessage, OutgoingMessage, OverlayStatus};

/// Número de clientes conectados (global, usado para status)
static CONNECTED_CLIENTS: AtomicUsize = AtomicUsize::new(0);

/// Evento interno del servidor WebSocket
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Un mensaje de chat llegó de un cliente
    Message(IncomingMessage),

    /// Un cliente se conectó
    ClientConnected(SocketAddr),

    /// Un cliente se desconectó
    ClientDisconnected(SocketAddr),
}

/// Servidor WebSocket del overlay
pub struct WebSocketServer {
    bind_addr: SocketAddr,
    event_tx: mpsc::UnboundedSender<WsEvent>,
    /// Broadcaster para mandar mensajes a todos los clientes (ej: status)
    broadcast_tx: broadcast::Sender<String>,
}

impl WebSocketServer {
    /// Crea un nuevo servidor WebSocket
    ///
    /// # Arguments
    /// * `bind_addr` - Dirección IP:puerto donde escuchar (ej: "127.0.0.1:9001")
    /// * `event_tx` - Canal donde enviar los eventos de mensajes recibidos
    pub fn new(bind_addr: &str, event_tx: mpsc::UnboundedSender<WsEvent>) -> anyhow::Result<Self> {
        let addr: SocketAddr = bind_addr
            .parse()
            .map_err(|e| anyhow::anyhow!("Dirección WebSocket inválida '{}': {}", bind_addr, e))?;

        let (broadcast_tx, _) = broadcast::channel(256);

        Ok(Self {
            bind_addr: addr,
            event_tx,
            broadcast_tx,
        })
    }

    /// Obtiene un sender del canal de broadcast (para enviar a todos los clientes)
    pub fn get_broadcast_sender(&self) -> broadcast::Sender<String> {
        self.broadcast_tx.clone()
    }

    /// Número de clientes actualmente conectados
    pub fn client_count() -> usize {
        CONNECTED_CLIENTS.load(Ordering::Relaxed)
    }

    /// Inicia el servidor en un task de Tokio
    pub async fn start(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await.map_err(|e| {
            anyhow::anyhow!(
                "No se pudo iniciar WebSocket en {}: {}",
                self.bind_addr,
                e
            )
        })?;

        println!("[WS] 🌐 WebSocket escuchando en ws://{}", self.bind_addr);
        println!("[WS] 📋 Protocolo: JSON sobre WebSocket");
        println!("[WS] 📖 Consulta la documentación del schema en src/transport/schema.rs");

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
                            eprintln!("[WS] Error con cliente {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[WS] Error aceptando conexión: {}", e);
                }
            }
        }
    }
}

/// Maneja una conexión WebSocket individual
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    event_tx: mpsc::UnboundedSender<WsEvent>,
    broadcast_tx: broadcast::Sender<String>,
    mut broadcast_rx: broadcast::Receiver<String>,
) -> anyhow::Result<()> {
    let ws_stream = accept_async(stream).await.map_err(|e| {
        anyhow::anyhow!("Error en handshake WebSocket con {}: {}", addr, e)
    })?;

    CONNECTED_CLIENTS.fetch_add(1, Ordering::Relaxed);
    println!(
        "[WS] ✅ Cliente conectado: {} (total: {})",
        addr,
        CONNECTED_CLIENTS.load(Ordering::Relaxed)
    );

    // Notificar conexión
    let _ = event_tx.send(WsEvent::ClientConnected(addr));

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Enviar mensaje de bienvenida
    let welcome = serde_json::to_string(&OutgoingMessage::Status(OverlayStatus {
        active_windows: 0,
        connected_clients: CONNECTED_CLIENTS.load(Ordering::Relaxed),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
    .unwrap_or_default();

    let _ = ws_tx.send(Message::Text(welcome)).await;

    // Task para reenviar broadcasts al cliente
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if ws_tx.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Procesar mensajes entrantes
    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                handle_text_message(&text, addr, &event_tx, &broadcast_tx).await;
            }
            Ok(Message::Ping(data)) => {
                // El protocolo WebSocket maneja pings automáticamente con tungstenite,
                // pero si queremos responder manualmente a mensajes "ping" de la app:
                let pong = serde_json::to_string(&OutgoingMessage::Pong).unwrap_or_default();
                let _ = broadcast_tx.send(pong);
            }
            Ok(Message::Close(_)) => {
                println!("[WS] Cliente {} cerró la conexión", addr);
                break;
            }
            Err(e) => {
                eprintln!("[WS] Error con cliente {}: {}", addr, e);
                break;
            }
            _ => {}
        }
    }

    // Limpiar
    send_task.abort();
    CONNECTED_CLIENTS.fetch_sub(1, Ordering::Relaxed);
    println!(
        "[WS] 👋 Cliente desconectado: {} (total: {})",
        addr,
        CONNECTED_CLIENTS.load(Ordering::Relaxed)
    );
    let _ = event_tx.send(WsEvent::ClientDisconnected(addr));

    Ok(())
}

/// Procesa un mensaje de texto recibido por WebSocket
async fn handle_text_message(
    text: &str,
    addr: SocketAddr,
    event_tx: &mpsc::UnboundedSender<WsEvent>,
    broadcast_tx: &broadcast::Sender<String>,
) {
    match IncomingMessage::parse_and_validate(text) {
        Ok(msg) => {
            // Responder con ACK si es un mensaje de chat
            if let IncomingMessage::ChatMessage(ref payload) = msg {
                let id = payload.get_or_generate_id();
                let ack = serde_json::to_string(&OutgoingMessage::Ack { id }).unwrap_or_default();
                let _ = broadcast_tx.send(ack);
            }

            // Enviar al sistema principal
            if let Err(e) = event_tx.send(WsEvent::Message(msg)) {
                eprintln!("[WS] Error reenviando mensaje al sistema: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[WS] Mensaje inválido de {}: {}", addr, e);

            // Notificar error al cliente
            let error_msg = serde_json::to_string(&OutgoingMessage::Error {
                code: "VALIDATION_ERROR".to_string(),
                message: e.to_string(),
            })
            .unwrap_or_default();

            let _ = broadcast_tx.send(error_msg);
        }
    }
}
