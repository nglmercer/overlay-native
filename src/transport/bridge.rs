//! Transport to Core bridge
//!
//! This module provides the connection between the transport layer (IPC/WebSocket)
//! and the core renderer. It converts validated transport messages to core elements
//! and forwards them to the renderer.
//!
//! The bridge now supports dynamic alert creation via [`AlertRegistry`]. 
//! You can register custom alert factories and use them to create alerts
//! based on incoming message types.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::core::{
    Alert, AlertBuilder, AlertContext, AlertRegistry, Badge, CoreRenderer, 
    MessageFilter, SharedAlertRegistry,
};
use crate::transport::schema::{BadgePayload, IncomingMessage};
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

    /// Alert registry for dynamic alert creation
    alert_registry: SharedAlertRegistry,
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
    /// Create a new transport bridge with default alert registry
    pub fn new(renderer: CoreRenderer) -> Self {
        Self {
            renderer: Arc::new(RwLock::new(renderer)),
            event_tx: None,
            alert_registry: SharedAlertRegistry::with_defaults(),
        }
    }

    /// Create with a custom renderer
    pub fn with_renderer(renderer: CoreRenderer) -> Self {
        Self::new(renderer)
    }

    /// Create with custom renderer and alert registry
    pub fn with_registry(renderer: CoreRenderer, registry: AlertRegistry) -> Self {
        Self {
            renderer: Arc::new(RwLock::new(renderer)),
            event_tx: None,
            alert_registry: SharedAlertRegistry::from_registry(registry),
        }
    }

    /// Set the event channel
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<BridgeEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get the renderer for external use
    pub fn get_renderer(&self) -> Arc<RwLock<CoreRenderer>> {
        self.renderer.clone()
    }

    /// Get the alert registry for managing custom alert factories
    pub fn get_alert_registry(&self) -> SharedAlertRegistry {
        self.alert_registry.clone()
    }

    /// Register a custom alert factory
    /// 
    /// # Example
    /// ```ignore
    /// bridge.register_alert_factory("my_custom", |ctx| {
    ///     AlertBuilder::new(&ctx.id)
    ///         .with_styled_text(ctx.get_or("title"), Some("#FF0000".to_string()), None)
    ///         .build()
    /// });
    /// ```
    pub async fn register_alert_factory<F>(&self, name: impl Into<String>, factory: F)
    where
        F: Fn(AlertContext) -> crate::core::Alert + Send + Sync + 'static,
    {
        self.alert_registry.register(name, factory).await;
    }

    /// Create and process an alert using the registry
    /// 
    /// # Example
    /// ```ignore
    /// let context = AlertContext::with_data("alert_1", [
    ///     ("username", "Streamer"),
    ///     ("content", "Hello world!"),
    ///     ("color", "#FF0000"),
    /// ]);
    /// bridge.create_and_process_alert("chat", context).await?;
    /// ```
    pub async fn create_and_process_alert(
        &self,
        alert_type: &str,
        context: AlertContext,
    ) -> Result<String, BridgeError> {
        let alert = self
            .alert_registry
            .create_or_default(alert_type, context)
            .await
            .ok_or_else(|| BridgeError::Config(format!("No factory registered for: {}", alert_type)))?;

        let id = alert.id.clone();

        let renderer = self.renderer.read().await;
        renderer
            .process_alert(alert)
            .await
            .map_err(|e| BridgeError::Render(e.to_string()))?;

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(BridgeEvent::MessageProcessed(id.clone()));
        }

        Ok(id)
    }

    /// Check if an alert factory is registered
    pub async fn has_alert_factory(&self, name: &str) -> bool {
        self.alert_registry.contains(name).await
    }

    /// Get list of registered alert types
    pub async fn get_registered_alerts(&self) -> Vec<String> {
        self.alert_registry.names().await
    }

    /// Process a WebSocket event and forward to renderer
    pub async fn handle_ws_event(&self, event: WsEvent) -> Result<(), BridgeError> {
        match event {
            WsEvent::Message(msg) => {
                let (msg_type, id, data) = msg.into_parts();

                // Skip system messages
                if msg_type == "ping" || msg_type == "status" {
                    return Ok(());
                }

                let context = AlertContext::from_value(id, data);
                self.create_and_process_alert(&msg_type, context).await?;
                Ok(())
            }
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
            alert_registry: self.alert_registry.clone(),
        }
    }
}
