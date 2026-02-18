//! Transport to Core bridge
//!
//! This module provides the connection between the transport layer (IPC/WebSocket)
//! and the core renderer. It converts validated transport messages to core elements
//! and forwards them to the renderer.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::core::{
    ChatMessageElement, CoreRenderer, EmoteElement, GiftElement, MessageFilter, OverlayElement,
};
use crate::transport::schema::IncomingMessage;
use crate::transport::websocket::WsEvent;

/// Bridge between transport layer and core renderer
pub struct TransportBridge {
    /// The core renderer
    renderer: Arc<RwLock<CoreRenderer>>,

    /// Channel to send events to the main application
    event_tx: Option<mpsc::UnboundedSender<BridgeEvent>>,
}

/// Events emitted by the bridge
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// A message was successfully processed
    MessageProcessed(String),

    /// A message was filtered out
    MessageFiltered(String),

    /// An error occurred
    Error(String),
}

impl TransportBridge {
    /// Create a new transport bridge
    pub fn new(renderer: CoreRenderer) -> Self {
        Self {
            renderer: Arc::new(RwLock::new(renderer)),
            event_tx: None,
        }
    }

    /// Create with a custom renderer
    pub fn with_renderer(renderer: CoreRenderer) -> Self {
        Self::new(renderer)
    }

    /// Set the event channel
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<BridgeEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get the renderer for external use
    pub fn get_renderer(&self) -> Arc<RwLock<CoreRenderer>> {
        self.renderer.clone()
    }

    /// Process a WebSocket event and forward to renderer
    pub async fn handle_ws_event(&self, event: WsEvent) -> Result<(), BridgeError> {
        match event {
            WsEvent::Message(msg) => self.handle_incoming_message(msg).await,
            WsEvent::ClientConnected(addr) => {
                println!("[BRIDGE] Client connected: {}", addr);
                Ok(())
            }
            WsEvent::ClientDisconnected(addr) => {
                println!("[BRIDGE] Client disconnected: {}", addr);
                Ok(())
            }
        }
    }

    /// Handle an incoming message from transport
    pub async fn handle_incoming_message(&self, msg: IncomingMessage) -> Result<(), BridgeError> {
        match msg {
            IncomingMessage::ChatMessage(payload) => {
                // Validate message
                payload
                    .validate()
                    .map_err(|e| BridgeError::ValidationError(e.to_string()))?;

                // Convert to core message
                let chat_msg: ChatMessageElement = payload.into();
                let id = chat_msg.id.clone();

                // Queue in renderer
                let renderer = self.renderer.read().await;
                renderer
                    .process_chat_message(chat_msg)
                    .await
                    .map_err(|e| BridgeError::RenderError(e.to_string()))?;

                // Emit event
                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Gift(payload) => {
                payload
                    .validate()
                    .map_err(|e| BridgeError::ValidationError(e.to_string()))?;

                let gift: GiftElement = payload.into();
                let id = gift.id.clone();

                let renderer = self.renderer.read().await;
                renderer
                    .process_gift(gift)
                    .await
                    .map_err(|e| BridgeError::RenderError(e.to_string()))?;

                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Emote(payload) => {
                payload
                    .validate()
                    .map_err(|e| BridgeError::ValidationError(e.to_string()))?;

                let emote: EmoteElement = payload.into();
                let id = emote.id.clone();

                let renderer = self.renderer.read().await;
                renderer
                    .process_emote(emote)
                    .await
                    .map_err(|e| BridgeError::RenderError(e.to_string()))?;

                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Ping => {
                // Ping is handled by the transport layer itself
                Ok(())
            }

            IncomingMessage::Status => {
                // Status request - handled by transport
                Ok(())
            }
        }
    }

    /// Update the message filter
    pub async fn update_filter(&self, filter: MessageFilter) {
        let renderer = self.renderer.read().await;
        renderer.update_filter(filter).await;
    }

    /// Get the current status
    pub async fn get_status(&self) -> BridgeStatus {
        let renderer = self.renderer.read().await;
        let status = renderer.get_status().await;

        BridgeStatus {
            active_elements: status.active_elements,
            queued_elements: status.queued_elements,
            total_processed: status.total_rendered,
        }
    }

    /// Clear all active elements
    pub async fn clear_all(&self) {
        let renderer = self.renderer.read().await;
        renderer.clear_all().await;
    }
}

/// Bridge status
#[derive(Debug, Clone)]
pub struct BridgeStatus {
    pub active_elements: usize,
    pub queued_elements: usize,
    pub total_processed: u64,
}

/// Bridge errors
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl Clone for TransportBridge {
    fn clone(&self) -> Self {
        Self {
            renderer: self.renderer.clone(),
            event_tx: self.event_tx.clone(),
        }
    }
}
