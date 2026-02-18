//! Platform-agnostic renderer
//!
//! This module provides the core rendering functionality for overlay elements.
//! It is completely independent of any streaming platform and only receives
//! normalized, validated messages from the transport layer.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::config::{
    AnimationSettings, CoreConfig, DisplaySettings, MessageFilter, ProcessingSettings,
    WindowSettings,
};
use super::message::{ChatMessageElement, GiftElement, OverlayElement};

/// Errors that can occur during rendering
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Window limit reached: {0}")]
    WindowLimitReached(usize),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Rendering failed: {0}")]
    RenderingFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Event emitted by the renderer
#[derive(Debug, Clone)]
pub enum RenderEvent {
    /// A new element should be displayed
    ElementQueued(Box<OverlayElement>),

    /// An element has finished its display duration
    ElementExpired(String),

    /// The renderer needs to redraw
    RedrawNeeded,

    /// Status update
    Status(RendererStatus),
}

/// Current status of the renderer
#[derive(Debug, Clone)]
pub struct RendererStatus {
    /// Number of active elements being displayed
    pub active_elements: usize,

    /// Number of elements queued
    pub queued_elements: usize,

    /// Total elements rendered since start
    pub total_rendered: u64,
}

/// The core renderer - platform-agnostic overlay display system
pub struct CoreRenderer {
    /// Configuration
    config: Arc<RwLock<CoreConfig>>,

    /// Message filter
    filter: Arc<RwLock<MessageFilter>>,

    /// Active overlay elements
    active_elements: Arc<RwLock<HashMap<String, OverlayElement>>>,

    /// Event sender for communicating with the main application
    event_tx: Option<mpsc::UnboundedSender<RenderEvent>>,

    /// Statistics
    stats: Arc<RwLock<RendererStats>>,
}

/// Renderer statistics
#[derive(Debug, Default)]
pub struct RendererStats {
    /// Total elements rendered
    pub total_rendered: u64,

    /// Elements filtered out
    pub total_filtered: u64,

    /// Elements that errored
    pub total_errors: u64,
}

impl CoreRenderer {
    /// Create a new core renderer with default configuration
    pub fn new() -> Self {
        Self::with_config(CoreConfig::default())
    }

    /// Create a new core renderer with custom configuration
    pub fn with_config(config: CoreConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            filter: Arc::new(RwLock::new(MessageFilter::default())),
            active_elements: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
            stats: Arc::new(RwLock::new(RendererStats::default())),
        }
    }

    /// Set the event channel for renderer events
    pub fn set_event_channel(&mut self, tx: mpsc::UnboundedSender<RenderEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> CoreConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config(&self, config: CoreConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// Update the message filter
    pub async fn update_filter(&self, filter: MessageFilter) {
        let mut f = self.filter.write().await;
        *f = filter;
    }

    /// Get current renderer status
    pub async fn get_status(&self) -> RendererStatus {
        let active = self.active_elements.read().await;
        let stats = self.stats.read().await;

        RendererStatus {
            active_elements: active.len(),
            queued_elements: 0, // TODO: implement queue
            total_rendered: stats.total_rendered,
        }
    }

    /// Process and queue an overlay element for display
    /// This is the main entry point for receiving elements from transport
    pub async fn queue_element(&self, element: OverlayElement) -> Result<String, RenderError> {
        // Apply filters based on element type
        let should_display = match &element {
            OverlayElement::ChatMessage(msg) => {
                let filter = self.filter.read().await;
                let badges: Vec<super::message::Badge> = msg.badges.clone();
                filter.accepts(&msg.content, &msg.username, &badges)
            }
            OverlayElement::Gift(_) | OverlayElement::Emote(_) => true,
        };

        if !should_display {
            let mut stats = self.stats.write().await;
            stats.total_filtered += 1;
            return Err(RenderError::InvalidMessage(
                "Message filtered out".to_string(),
            ));
        }

        // Get element ID
        let element_id = match &element {
            OverlayElement::ChatMessage(msg) => msg.id.clone(),
            OverlayElement::Gift(gift) => gift.id.clone(),
            OverlayElement::Emote(emote) => emote.id.clone(),
        };

        // Check window limit
        let config = self.config.read().await;
        let max_windows = config.window.max_windows;
        let mut active = self.active_elements.write().await;

        if active.len() >= max_windows {
            return Err(RenderError::WindowLimitReached(max_windows));
        }

        // Store the element
        active.insert(element_id.clone(), element.clone());

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_rendered += 1;
        }

        // Emit event
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(RenderEvent::ElementQueued(element));
        }

        // Schedule removal after duration
        let active_elements = self.active_elements.clone();
        let event_tx = self.event_tx.clone();
        let element_id_clone = element_id.clone();
        let duration = config.window.message_duration();

        tokio::spawn(async move {
            tokio::time::sleep(duration).await;

            let mut elements = active_elements.write().await;
            elements.remove(&element_id_clone);

            if let Some(ref tx) = event_tx {
                let _ = tx.send(RenderEvent::ElementExpired(element_id_clone));
            }
        });

        Ok(element_id)
    }

    /// Process a chat message
    pub async fn process_chat_message(
        &self,
        message: ChatMessageElement,
    ) -> Result<String, RenderError> {
        self.queue_element(OverlayElement::ChatMessage(message))
            .await
    }

    /// Process a gift event
    pub async fn process_gift(&self, gift: GiftElement) -> Result<String, RenderError> {
        self.queue_element(OverlayElement::Gift(gift)).await
    }

    /// Process an emote event
    pub async fn process_emote(
        &self,
        emote: super::message::EmoteElement,
    ) -> Result<String, RenderError> {
        self.queue_element(OverlayElement::Emote(emote)).await
    }

    /// Get all active elements
    pub async fn get_active_elements(&self) -> Vec<OverlayElement> {
        let elements = self.active_elements.read().await;
        elements.values().cloned().collect()
    }

    /// Clear all active elements
    pub async fn clear_all(&self) {
        let mut elements = self.active_elements.write().await;
        elements.clear();

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(RenderEvent::RedrawNeeded);
        }
    }

    /// Get the window settings
    pub async fn get_window_settings(&self) -> WindowSettings {
        self.config.read().await.window.clone()
    }

    /// Get the animation settings
    pub async fn get_animation_settings(&self) -> AnimationSettings {
        self.config.read().await.animation.clone()
    }

    /// Get the display settings
    pub async fn get_display_settings(&self) -> DisplaySettings {
        self.config.read().await.display.clone()
    }

    /// Get the processing settings
    pub async fn get_processing_settings(&self) -> ProcessingSettings {
        self.config.read().await.processing.clone()
    }
}

impl Default for CoreRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CoreRenderer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            filter: self.filter.clone(),
            active_elements: self.active_elements.clone(),
            event_tx: self.event_tx.clone(),
            stats: self.stats.clone(),
        }
    }
}

// Re-export for convenience
pub use super::message::{Badge, Emote, TextPosition};
