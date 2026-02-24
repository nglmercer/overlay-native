//! Alert builder for creating complex overlay elements
//!
//! This module provides a fluent builder API for creating `Alert` objects
//! with various components like text, images, and badges.

use super::message::{Alert, AlertComponent, AlertStyle, Layout};
use std::time::SystemTime;

/// Builder for creating overlay Alerts
pub struct AlertBuilder {
    id: String,
    components: Vec<AlertComponent>,
    style: AlertStyle,
    layout: Layout,
    duration: Option<u64>,
}

impl AlertBuilder {
    /// Create a new AlertBuilder with a unique ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            components: Vec::new(),
            style: AlertStyle::default(),
            layout: Layout::Vertical,
            duration: None,
        }
    }

    /// Add a text component
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.components.push(AlertComponent::Text {
            content: text.into(),
            color: None,
            weight: None,
            style: None,
            size: None,
        });
        self
    }

    /// Add a styled text component
    pub fn with_styled_text(
        mut self,
        text: impl Into<String>,
        color: Option<String>,
        weight: Option<String>,
        style: Option<String>,
    ) -> Self {
        self.components.push(AlertComponent::Text {
            content: text.into(),
            color,
            weight,
            style,
            size: None,
        });
        self
    }

    /// Add an image component
    pub fn with_image(mut self, url: impl Into<String>) -> Self {
        self.components.push(AlertComponent::Image {
            url: url.into(),
            width: None,
            height: None,
            is_animated: false,
        });
        self
    }

    /// Add a badge component
    pub fn with_badge(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        url: Option<String>,
    ) -> Self {
        self.components.push(AlertComponent::Badge {
            id: id.into(),
            name: name.into(),
            url,
        });
        self
    }

    /// Set the layout of the alert
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Set a custom style for the alert
    pub fn with_style(mut self, style: AlertStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the display duration in seconds
    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration = Some(seconds);
        self
    }

    /// Build the Alert
    pub fn build(self) -> Alert {
        Alert {
            id: self.id,
            components: self.components,
            style: self.style,
            layout: self.layout,
            timestamp: SystemTime::now(),
            duration: self.duration,
        }
    }
}
