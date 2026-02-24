//! Transport to Core bridge
//!
//! This module provides the connection between the transport layer (IPC/WebSocket)
//! and the core renderer. It converts validated transport messages to core elements
//! and forwards them to the renderer.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};


use crate::core::{
    Badge, ChatMessageElement, CoreRenderer, GiftElement, GiftType,
    ImageElement, MessageFilter,
};
use crate::transport::schema::{
    BadgePayload, ChatMessagePayload, GiftPayload, ImagePayload, IncomingMessage,
};
use crate::transport::websocket::WsEvent;

// Conversion logic moved from core to bridge (separation of concerns)

impl From<ChatMessagePayload> for ChatMessageElement {
    fn from(payload: ChatMessagePayload) -> Self {
        Self {
            id: payload.get_or_generate_id(),
            username: payload.username.clone(),
            display_name: payload.display_name.clone(),
            content: payload.content.clone(),
            user_color: payload.user_color.clone(),
            badges: payload.badges.into_iter().map(|b| b.into()).collect(),
            platform: payload.platform.clone(),
            timestamp: std::time::SystemTime::now(),
            metadata: payload.metadata,
        }
    }
}

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

impl From<GiftPayload> for GiftElement {
    fn from(payload: GiftPayload) -> Self {
        let gift_type = match payload.gift_type {
            crate::transport::schema::GiftType::Subscription => GiftType::Subscription,
            crate::transport::schema::GiftType::GiftSubscription => GiftType::GiftSubscription,
            crate::transport::schema::GiftType::Bits => GiftType::Bits,
            crate::transport::schema::GiftType::Cheer => GiftType::Cheer,
            crate::transport::schema::GiftType::Donation => GiftType::Donation,
            crate::transport::schema::GiftType::Other(s) => GiftType::Other(s),
        };

        Self {
            id: format!(
                "gift_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            from_user: payload.from_user,
            to_user: payload.to_user,
            gift_type,
            amount: payload.amount,
            tier: payload.tier,
            message: payload.message,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

impl From<ImagePayload> for ImageElement {
    fn from(payload: ImagePayload) -> Self {
        Self {
            id: payload.id.clone(),
            name: payload.name.clone(),
            url: payload.url.clone(),
            is_animated: payload.is_animated,
            width: payload.width,
            height: payload.height,
            sender: payload.sender,
            timestamp: std::time::SystemTime::now(),
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
                // Validate message
                payload
                    .validate()
                    .map_err(|e| BridgeError::Validation(e.to_string()))?;

                // Convert to core message
                let chat_msg: ChatMessageElement = payload.into();
                let id = chat_msg.id.clone();

                // Queue in renderer
                let renderer = self.renderer.read().await;
                renderer
                    .process_chat_message(chat_msg)
                    .await
                    .map_err(|e| BridgeError::Render(e.to_string()))?;

                // Emit event
                if let Some(ref tx) = self.event_tx {
                    let _ = tx.send(BridgeEvent::MessageProcessed(id));
                }

                Ok(())
            }

            IncomingMessage::Gift(payload) => {
                payload
                    .validate()
                    .map_err(|e| BridgeError::Validation(e.to_string()))?;

                let gift: GiftElement = payload.into();
                let id = gift.id.clone();

                let renderer = self.renderer.read().await;
                renderer
                    .process_gift(gift)
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

                let image: ImageElement = payload.into();
                let id = image.id.clone();

                let renderer = self.renderer.read().await;
                renderer
                    .process_image(image)
                    .await
                    .map_err(|e| BridgeError::Render(e.to_string()))?;

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
