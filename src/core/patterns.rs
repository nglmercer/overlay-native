//! Reusable alert patterns and templates
//!
//! This module provides predefined patterns for common overlay elements
//! ensuring consistency across different platforms and providers.

use super::builder::AlertBuilder;
use super::message::{Alert, AlertComponent, AlertStyle, Badge, Layout};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertTemplate {
    pub name: String,
    pub layout: Layout,
    pub components: Vec<AlertComponent>,
    pub style: AlertStyle,
    pub default_duration: Option<u64>,
}

#[derive(Clone)]
pub struct TemplateRegistry {
    templates: HashMap<String, AlertTemplate>,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn load_from_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        if !path.as_ref().exists() {
            return Err(format!(
                "Templates directory not found: {:?}",
                path.as_ref()
            ));
        }

        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let template: AlertTemplate =
                    serde_json::from_str(&content).map_err(|e| e.to_string())?;
                println!("[TEMPLATES] Loaded template: {}", template.name);
                self.templates.insert(template.name.clone(), template);
            }
        }
        Ok(())
    }

    pub fn render(
        &self,
        name: &str,
        id: String,
        context: &HashMap<String, String>,
    ) -> Option<Alert> {
        let template = self.templates.get(name)?;
        let mut builder = AlertBuilder::new(id).with_layout(template.layout.clone());

        for component in &template.components {
            match component {
                AlertComponent::Text {
                    content,
                    color,
                    weight,
                    style,
                    size: _,
                } => {
                    let mut final_content = content.clone();
                    for (key, value) in context {
                        final_content = final_content.replace(&format!("{{{{{}}}}}", key), value);
                    }

                    // Also interpolate color if it's a placeholder
                    let final_color = color.as_ref().map(|c| {
                        let mut cc = c.clone();
                        for (key, value) in context {
                            cc = cc.replace(&format!("{{{{{}}}}}", key), value);
                        }
                        cc
                    });

                    builder = builder.with_styled_text(
                        final_content,
                        final_color,
                        weight.clone(),
                        style.clone(),
                    );
                }
                AlertComponent::Image {
                    url,
                    width: _,
                    height: _,
                    is_animated: _,
                } => {
                    let mut final_url = url.clone();
                    for (key, value) in context {
                        final_url = final_url.replace(&format!("{{{{{}}}}}", key), value);
                    }
                    builder = builder.with_image(final_url);
                }
                AlertComponent::Badge { id, name, url } => {
                    // Badges can also be interpolated if needed, but usually passed via context
                    // For now handle simple case
                    builder = builder.with_badge(id.clone(), name.clone(), url.clone());
                }
            }
        }

        builder = builder.with_style(template.style.clone());
        if let Some(d) = template.default_duration {
            builder = builder.with_duration(d);
        }

        Some(builder.build())
    }

    pub fn register_defaults(&mut self) {
        // Register modern_chat
        self.templates.insert(
            "modern_chat".to_string(),
            AlertTemplate {
                name: "modern_chat".to_string(),
                layout: Layout::Horizontal,
                components: vec![
                    // Badges would be added dynamically in the old system,
                    // but here we can define a placeholder for them if we want,
                    // or just rely on the fact that badge component exists.
                    AlertComponent::Text {
                        content: "{{username}}: ".to_string(),
                        color: Some("{{color}}".to_string()),
                        weight: Some("bold".to_string()),
                        style: None,
                        size: None,
                    },
                    AlertComponent::Text {
                        content: "{{content}}".to_string(),
                        color: None,
                        weight: None,
                        style: None,
                        size: None,
                    },
                ],
                style: AlertStyle {
                    padding: Some(12),
                    border_radius: Some(8),
                    background_color: Some("rgba(0, 0, 0, 0.7)".to_string()),
                    ..Default::default()
                },
                default_duration: None,
            },
        );

        // Add other defaults as needed...
    }

    pub async fn register_to_alert_registry(
        &self,
        alert_registry: &crate::core::SharedAlertRegistry,
    ) {
        for name in self.templates.keys() {
            let name_clone = name.clone();
            let self_clone = self.clone();
            alert_registry
                .register(name_clone.clone(), move |ctx| {
                    self_clone
                        .render(&name_clone, ctx.id.clone(), &ctx.data_as_strings())
                        .unwrap_or_else(|| {
                            // Fallback if template disappeared (unlikely)
                            AlertBuilder::new(&ctx.id)
                                .with_text("Template error")
                                .build()
                        })
                })
                .await;
        }
    }
}

pub struct AlertTemplates;

impl AlertTemplates {
    /// A premium-looking chat message pattern
    pub fn modern_chat(
        id: String,
        username: String,
        content: String,
        color: Option<String>,
        badges: Vec<Badge>,
    ) -> Alert {
        let mut builder = AlertBuilder::new(id).with_layout(Layout::Horizontal);

        // Add badges first
        for badge in badges {
            builder = builder.with_badge(badge.id, badge.name, badge.url);
        }

        // Username and message in a specific style
        builder
            .with_styled_text(
                format!("{}: ", username),
                color,
                Some("bold".to_string()),
                None,
            )
            .with_text(content)
            .with_style(AlertStyle {
                padding: Some(12),
                border_radius: Some(8),
                background_color: Some("rgba(0, 0, 0, 0.7)".to_string()),
                ..Default::default()
            })
            .build()
    }

    /// A notification banner (centered, big text)
    pub fn notification_banner(id: String, title: String, subtitle: Option<String>) -> Alert {
        let mut builder = AlertBuilder::new(id)
            .with_layout(Layout::Vertical)
            .with_styled_text(
                title,
                Some("#FFFFFF".to_string()),
                Some("bold".to_string()),
                None,
            );

        if let Some(sub) = subtitle {
            builder = builder.with_styled_text(sub, Some("#CCCCCC".to_string()), None, None);
        }

        builder
            .with_style(AlertStyle {
                padding: Some(20),
                background_color: Some("linear-gradient(90deg, #6441a5, #2a0845)".to_string()), // Twitch colors placeholder
                border_radius: Some(0), // Full width banner style
                ..Default::default()
            })
            .build()
    }

    /// Achievement/Goal reached pattern
    pub fn achievement(id: String, title: String, icon_url: String) -> Alert {
        AlertBuilder::new(id)
            .with_layout(Layout::Horizontal)
            .with_image(icon_url)
            .with_styled_text(
                " UNLOCKED: ",
                Some("#ffd700".to_string()),
                Some("bold".to_string()),
                None,
            )
            .with_text(title)
            .with_duration(10)
            .with_style(AlertStyle {
                border_color: Some("#ffd700".to_string()),
                border_radius: Some(25),
                ..Default::default()
            })
            .build()
    }

    /// A premium gift pattern with custom text support (e.g. Spanish)
    pub fn premium_gift(
        id: String,
        username: String,
        amount: String,
        gift_name: String,
        image_url: Option<String>,
    ) -> Alert {
        let mut builder = AlertBuilder::new(id).with_layout(Layout::Vertical);

        if let Some(url) = image_url {
            builder = builder.with_image(url);
        }

        builder
            .with_styled_text(
                format!("{} regalo y envió", username),
                Some("#FFD700".to_string()), // Gold
                Some("bold".to_string()),
                None,
            )
            .with_styled_text(
                format!("{} {}", amount, gift_name),
                Some("#FFFFFF".to_string()),
                None,
                Some("italic".to_string()),
            )
            .with_style(AlertStyle {
                padding: Some(24),
                border_radius: Some(15),
                background_color: Some(
                    "linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%)".to_string(),
                ),
                border_color: Some("#FFD700".to_string()),
                ..Default::default()
            })
            .with_duration(12)
            .build()
    }
}
