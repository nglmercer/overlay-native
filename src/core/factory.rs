//! Dynamic Alert Factory and Registry
//!
//! This module provides a flexible system for registering and creating alerts
//! at runtime. Instead of hardcoding alert creation in the bridge, you can
//! register custom alert factories that can be invoked dynamically.
//!
//! ## Usage
//!
//! ```rust
//! use overlay_native::core::{AlertRegistry, AlertFactory, AlertContext};
//!
//! // Create a registry
//! let mut registry = AlertRegistry::new();
//!
//! // Register a custom alert type
//! registry.register("custom_alert", |ctx| {
//!     AlertBuilder::new(&ctx.id)
//!         .with_styled_text(ctx.get::<String>("title").unwrap_or_default(), Some("#FF0000".to_string()), Some("bold".to_string()))
//!         .with_text(ctx.get::<String>("message").unwrap_or_default())
//!         .build()
//! });
//!
//! // Use it later
//! let alert = registry.create("custom_alert", AlertContext::new("my_id", [("title", "Hello"), ("message", "World")]));
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::builder::AlertBuilder;
use super::message::Alert;

/// Context passed to alert factories containing message data
#[derive(Debug, Clone)]
pub struct AlertContext {
    /// Unique identifier for the alert
    pub id: String,

    /// Key-value data from the incoming message
    data: HashMap<String, serde_json::Value>,
}

impl AlertContext {
    /// Create a new context with an ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: HashMap::new(),
        }
    }

    /// Create from a slice of key-value pairs
    pub fn with_data<K, V>(id: impl Into<String>, data: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        let mut ctx = Self::new(id);
        for (k, v) in data {
            ctx.data.insert(k.into(), v.into());
        }
        ctx
    }

    /// Get a string value from context
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).and_then(|v| v.as_str().map(String::from))
    }

    /// Get a typed value from context
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a value as a specific type, or use a default
    pub fn get_or<T: serde::de::DeserializeOwned + Default>(&self, key: &str) -> T {
        self.get(key).unwrap_or_default()
    }

    /// Insert a value into context
    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.data.insert(key.into(), value.into());
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get the inner data map (for iteration)
    pub fn data(&self) -> &HashMap<String, serde_json::Value> {
        &self.data
    }

    /// Create context from a JSON value
    pub fn from_value(id: impl Into<String>, value: serde_json::Value) -> Self {
        let mut ctx = Self::new(id);
        if let serde_json::Value::Object(map) = value {
            for (k, v) in map {
                ctx.data.insert(k, v);
            }
        }
        ctx
    }
}

/// Factory trait for creating alerts
/// Implement this trait to create custom alert types
pub trait AlertFactory: Send + Sync {
    /// Create an alert from the given context
    fn create(&self, context: AlertContext) -> Alert;

    /// Get the name/type of this factory
    fn name(&self) -> &str;

    /// Get a description of what this factory creates
    fn description(&self) -> Option<&str>;
}

/// Wrapper for implementing AlertFactory on closures
struct ClosureAdapter<F> {
    name: String,
    description: Option<String>,
    closure: F,
}

impl<F> AlertFactory for ClosureAdapter<F>
where
    F: Fn(AlertContext) -> Alert + Send + Sync + 'static,
{
    fn create(&self, context: AlertContext) -> Alert {
        (self.closure)(context)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Registry for alert factories
/// Allows dynamic registration and creation of alerts at runtime
#[derive(Default)]
pub struct AlertRegistry {
    /// Map of factory name to factory
    factories: HashMap<String, Box<dyn AlertFactory>>,

    /// Default factory to use when no specific factory is found
    default_factory: Option<String>,
}

impl AlertRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new alert factory with a closure
    /// 
    /// # Example
    /// ```rust
    /// registry.register("chat_message", |ctx| {
    ///     Alert::chat(
    ///         ctx.id.clone(),
    ///         ctx.get_or("username"),
    ///         ctx.get_or("content"),
    ///         ctx.get("user_color"),
    ///         vec![],
    ///     )
    /// });
    /// ```
    pub fn register<F>(&mut self, name: impl Into<String>, factory: F) -> &mut Self
    where
        F: Fn(AlertContext) -> Alert + Send + Sync + 'static,
    {
        let name = name.into();
        self.factories.insert(
            name.clone(),
            Box::new(ClosureAdapter {
                name,
                description: None,
                closure: factory,
            }),
        );
        self
    }

    /// Register a factory with metadata
    pub fn register_with_meta<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        factory: F,
    ) -> &mut Self
    where
        F: Fn(AlertContext) -> Alert + Send + Sync + 'static,
    {
        let name = name.into();
        self.factories.insert(
            name.clone(),
            Box::new(ClosureAdapter {
                name,
                description: Some(description.into()),
                closure: factory,
            }),
        );
        self
    }

    /// Set the default factory to use when no specific factory is found
    pub fn set_default(&mut self, name: impl Into<String>) -> &mut Self {
        self.default_factory = Some(name.into());
        self
    }

    /// Check if a factory is registered
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    /// Create an alert using a registered factory
    /// Returns None if the factory doesn't exist
    pub fn create(&self, name: &str, context: AlertContext) -> Option<Alert> {
        self.factories.get(name).map(|f| f.create(context))
    }

    /// Create an alert, falling back to default if not found
    /// Returns None if neither specific nor default factory exists
    pub fn create_or_default(&self, name: &str, context: AlertContext) -> Option<Alert> {
        if let Some(factory) = self.factories.get(name) {
            return Some(factory.create(context));
        }

        if let Some(default_name) = &self.default_factory {
            if let Some(factory) = self.factories.get(default_name) {
                return Some(factory.create(context));
            }
        }

        None
    }

    /// Get all registered factory names (owned strings)
    pub fn names(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Get factory information
    pub fn info(&self, name: &str) -> Option<FactoryInfo> {
        self.factories.get(name).map(|f| FactoryInfo {
            name: f.name().to_string(),
            description: f.description().map(String::from),
        })
    }

    /// Get info for all factories
    pub fn all_info(&self) -> Vec<FactoryInfo> {
        self.factories
            .values()
            .map(|f| FactoryInfo {
                name: f.name().to_string(),
                description: f.description().map(String::from),
            })
            .collect()
    }

    /// Unregister a factory
    pub fn unregister(&mut self, name: &str) -> bool {
        self.factories.remove(name).is_some()
    }

    /// Clear all registered factories
    pub fn clear(&mut self) {
        self.factories.clear();
        self.default_factory = None;
    }

    /// Get the number of registered factories
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Register the default alert factories (chat_message, gift, image, custom)
    pub fn with_defaults(mut self) -> Self {
        // Register default chat alert
        self.register("chat_message", |ctx| {
            // Support both 'color' and 'user_color' for flexibility
            let color = ctx.get::<String>("user_color").or_else(|| ctx.get::<String>("color"));
            
            Alert::chat(
                ctx.id.clone(),
                ctx.get_or("username"),
                ctx.get_or("content"),
                color,
                vec![], // Badges handled separately or could be in context
            )
        });

        // Register default gift alert
        self.register("gift", |ctx| {
            let from = ctx.get::<String>("from_user").or_else(|| ctx.get::<String>("from")).unwrap_or_else(|| "Anonymous".to_string());
            
            // Reconstruct gift description if types are provided
            let gift_desc = ctx.get::<String>("gift_desc").or_else(|| {
                ctx.get::<String>("gift_type").map(|t| {
                    let amount = ctx.get::<u32>("amount").unwrap_or(1);
                    format!("{} x{}", t, amount)
                })
            }).unwrap_or_else(|| "a gift".to_string());

            let message = ctx.get_string("message");
            
            Alert::gift(ctx.id.clone(), from, gift_desc, message)
        });

        // Register default image alert
        self.register("image", |ctx| {
            let url = ctx.get_or::<String>("url");
            let name = ctx.get_or::<String>("name");
            
            AlertBuilder::new(&ctx.id)
                .with_image(url)
                .with_styled_text(name, None, Some("italic".to_string()))
                .build()
        });

        // Register a custom/fallback alert type
        self.register("custom", |ctx| {
            let mut builder = AlertBuilder::new(&ctx.id);
            
            // Add any text components found in the data
            for (key, value) in ctx.data() {
                if let Some(text) = value.as_str() {
                    if key != "id" && key != "type" {
                        builder = builder.with_styled_text(format!("{}: {}", key, text), None, None);
                    }
                }
            }
            
            builder.build()
        });

        self.set_default("custom");
        self
    }
}

/// Information about a registered factory
#[derive(Debug, Clone)]
pub struct FactoryInfo {
    pub name: String,
    pub description: Option<String>,
}

/// Thread-safe wrapper for AlertRegistry
#[derive(Clone, Default)]
pub struct SharedAlertRegistry {
    inner: Arc<RwLock<AlertRegistry>>,
}

impl SharedAlertRegistry {
    /// Create a new shared registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AlertRegistry::new())),
        }
    }

    /// Create with default factories pre-registered
    pub fn with_defaults() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AlertRegistry::new().with_defaults())),
        }
    }

    /// Create from an existing AlertRegistry
    pub fn from_registry(registry: AlertRegistry) -> Self {
        Self {
            inner: Arc::new(RwLock::new(registry)),
        }
    }

    /// Register a new alert factory
    pub async fn register<F>(&self, name: impl Into<String>, factory: F)
    where
        F: Fn(AlertContext) -> Alert + Send + Sync + 'static,
    {
        self.inner.write().await.register(name, factory);
    }

    /// Create an alert using a registered factory
    pub async fn create(&self, name: &str, context: AlertContext) -> Option<Alert> {
        self.inner.read().await.create(name, context)
    }

    /// Create with fallback to default
    pub async fn create_or_default(&self, name: &str, context: AlertContext) -> Option<Alert> {
        self.inner.read().await.create_or_default(name, context)
    }

    /// Check if a factory exists
    pub async fn contains(&self, name: &str) -> bool {
        self.inner.read().await.contains(name)
    }

    /// Get all registered names (owned strings)
    pub async fn names(&self) -> Vec<String> {
        self.inner.read().await.names()
    }

    /// Update the entire registry
    pub async fn set_registry(&self, registry: AlertRegistry) {
        let mut inner = self.inner.write().await;
        *inner = registry;
    }
}

/// Builder extension trait for more fluent context creation
pub trait AlertContextBuilder {
    fn with_id(self, id: impl Into<String>) -> Self;
    fn with(self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self;
    fn build(self) -> AlertContext;
}

impl AlertContextBuilder for AlertContext {
    fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    fn with(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    fn build(self) -> AlertContext {
        self
    }
}

/// Convenient function to create an alert context
pub fn alert_context(id: impl Into<String>) -> AlertContext {
    AlertContext::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = AlertRegistry::new();

        registry.register("test", |ctx| {
            AlertBuilder::new(&ctx.id)
                .with_text("Test alert")
                .build()
        });

        let context = AlertContext::new("alert_1");
        let alert = registry.create("test", context).unwrap();

        assert_eq!(alert.id, "alert_1");
    }

    #[test]
    fn test_registry_with_data() {
        let mut registry = AlertRegistry::new();

        registry.register("greeting", |ctx| {
            let name = ctx.get_or::<String>("name");
            AlertBuilder::new(&ctx.id)
                .with_styled_text(format!("Hello, {}!", name), Some("#00FF00".to_string()), Some("bold".to_string()))
                .build()
        });

        let context = AlertContext::with_data("greet_1", [("name", "World")]);
        let alert = registry.create("greeting", context).unwrap();

        assert_eq!(alert.id, "greet_1");
    }

    #[test]
    fn test_default_factory() {
        let mut registry = AlertRegistry::new();

        registry
            .register("special", |ctx| {
                AlertBuilder::new(&ctx.id).with_text("Special!").build()
            })
            .set_default("default");

        registry.register("default", |ctx| {
            AlertBuilder::new(&ctx.id).with_text("Default").build()
        });

        // Should use "special" factory
        let alert1 = registry.create_or_default("special", AlertContext::new("1")).unwrap();
        
        // Should fall back to "default" factory
        let alert2 = registry.create_or_default("nonexistent", AlertContext::new("2")).unwrap();

        assert_eq!(alert1.components.len(), 1);
        assert_eq!(alert2.components.len(), 1);
    }

    #[test]
    fn test_with_defaults() {
        let registry = AlertRegistry::new().with_defaults();

        assert!(registry.contains("chat"));
        assert!(registry.contains("gift"));
        assert!(registry.contains("image"));
        assert!(registry.contains("custom"));
    }
}
