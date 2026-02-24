//! Transport to Core bridge
//!
//! This module provides the connection between the transport layer (IPC/WebSocket)
//! and the core renderer. It converts validated transport messages to core elements
//! and forwards them to the renderer.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};


use crate::core::{
    Alert, AlertBuilder, Badge, CoreRenderer, MessageFilter, OverlayElement,
};
use crate::transport::schema::{
    BadgePayload, ChatMessagePayload, GiftPayload, ImagePayload, IncomingMessage,
};
use crate::transport::websocket::WsEvent;

// Conversion logic updated to target Alert directly

impl From<BadgePayload> for Badge {
    fn from(payload: BadgePayload) -> Self {
        Self {
            id: payload.id.clone(),
            name: payload.name.clone(),
            url: payload.url.clone(),
            title: payload.title.clone(),
        }
    }
}

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
            WsEvent::Message(msg) => self.handle_incoming_message(*msg).await,
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
                payload
                    .validate()
                    .map_err(|e| BridgeError::Validation(e.to_string()))?;

                let id = payload.get_or_generate_id();
                let badges: Vec<Badge> = payload.badges.into_iter().map(|b| b.into()).collect();
                
                // Map to Generic Alert
                let alert = Alert::chat(
                    id.clone(),
                    payload.username,
                    payload.content,
                    payload.user_color,
                    badges,
                );

                let renderer = self.renderer.read().await;
                renderer
                    .process_alert(alert)
                    .await
                    .map_err(|e| BridgeError::Render(e.to_string()))?;

                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Gift(payload) => {
                payload
                    .validate()
                    .map_err(|e| BridgeError::Validation(e.to_string()))?;

                let id = format!(
                    "gift_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                );
                
                let gift_desc = match payload.gift_type {
                    crate::transport::schema::GiftType::Subscription => format!("a {} month sub", payload.amount.unwrap_or(1)),
                    crate::transport::schema::GiftType::GiftSubscription => format!("a gifted sub"),
                    crate::transport::schema::GiftType::Bits => format!("{} bits", payload.amount.unwrap_or(1)),
                    crate::transport::schema::GiftType::Cheer => format!("a cheer"),
                    crate::transport::schema::GiftType::Donation => format!("a donation"),
                    crate::transport::schema::GiftType::Other(s) => s,
                };

                let alert = Alert::gift(id.clone(), payload.from_user, gift_desc, payload.message);

                let renderer = self.renderer.read().await;
                renderer
                    .process_alert(alert)
                    .await
                    .map_err(|e| BridgeError::Render(e.to_string()))?;

                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Image(payload) => {
                payload
                    .validate()
                    .map_err(|e| BridgeError::Validation(e.to_string()))?;

                let id = payload.id.clone();
                let alert = AlertBuilder::new(&id)
                    .with_image(payload.url)
                    .with_styled_text(payload.name, None, Some("italic".to_string()))
                    .build();

                let renderer = self.renderer.read().await;
                renderer
                    .process_alert(alert)
                    .await
                    .map_err(|e| BridgeError::Render(e.to_string()))?;

                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Ping | IncomingMessage::Status => Ok(()),
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
    Validation(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl Clone for TransportBridge {
    fn clone(&self) -> Self {
        Self {
            renderer: self.renderer.clone(),
            event_tx: self.event_tx.clone(),
        }
    }
}
