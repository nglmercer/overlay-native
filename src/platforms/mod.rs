pub mod base;
pub mod kick;
pub mod twitch;
pub mod youtube;

pub use base::*;
pub use kick::*;
pub use twitch::*;
pub use youtube::*;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fábrica de plataformas
pub struct PlatformFactory {
    platforms: HashMap<String, Arc<dyn PlatformCreator + Send + Sync>>,
}

impl PlatformFactory {
    pub fn new() -> Self {
        let mut factory = Self {
            platforms: HashMap::new(),
        };

        // Registrar plataformas por defecto
        factory.register_platform("twitch".to_string(), Arc::new(TwitchCreator));
        // factory.register_platform("youtube".to_string(), Arc::new(YouTubeCreator));
        // factory.register_platform("kick".to_string(), Arc::new(KickCreator));

        factory
    }

    pub fn register_platform(&mut self, name: String, creator: Arc<dyn PlatformCreator>) {
        self.platforms.insert(name, creator);
    }

    /// Crea una instancia de plataforma
    pub async fn create_platform(
        &self,
        platform_type: &str,
        config: crate::config::PlatformConfig,
    ) -> Result<
        Box<
            dyn crate::connection::StreamingPlatform<Error = PlatformWrapperError>
                + Send
                + Sync
                + 'static,
        >,
        PlatformError,
    > {
        let creator = self
            .platforms
            .get(platform_type)
            .ok_or_else(|| PlatformError::UnsupportedPlatform(platform_type.to_string()))?;

        creator.create(config).await
    }

    pub fn list_supported_platforms(&self) -> Vec<String> {
        self.platforms.keys().cloned().collect()
    }
}

impl Default for PlatformFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait para crear instancias de plataformas
#[async_trait]
pub trait PlatformCreator: Send + Sync {
    async fn create(
        &self,
        config: crate::config::PlatformConfig,
    ) -> Result<
        Box<
            dyn crate::connection::StreamingPlatform<Error = PlatformWrapperError>
                + Send
                + Sync
                + 'static,
        >,
        PlatformError,
    >;

    fn platform_name(&self) -> &str;

    fn required_credentials(&self) -> Vec<&'static str>;

    async fn validate_credentials(
        &self,
        credentials: &crate::config::Credentials,
    ) -> Result<bool, PlatformError>;
}

/// Errores de plataforma
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Plataforma no soportada: {0}")]
    UnsupportedPlatform(String),

    #[error("Error de configuración: {0}")]
    ConfigError(String),

    #[error("Error de autenticación: {0}")]
    AuthError(String),

    #[error("Error de conexión: {0}")]
    ConnectionError(String),

    #[error("Error de API: {0}")]
    ApiError(String),

    #[error("Error de parsing: {0}")]
    ParseError(String),
}

/// Gestor de credenciales seguro
#[derive(Clone)]
pub struct CredentialManager {
    credentials: Arc<RwLock<HashMap<String, crate::config::Credentials>>>,
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store_credentials(
        &self,
        platform: String,
        credentials: crate::config::Credentials,
    ) {
        let mut creds = self.credentials.write().await;
        creds.insert(platform, credentials);
    }

    pub async fn get_credentials(&self, platform: &str) -> Option<crate::config::Credentials> {
        let creds = self.credentials.read().await;
        creds.get(platform).cloned()
    }

    pub async fn remove_credentials(&self, platform: &str) -> bool {
        let mut creds = self.credentials.write().await;
        creds.remove(platform).is_some()
    }

    pub async fn list_platforms(&self) -> Vec<String> {
        let creds = self.credentials.read().await;
        creds.keys().cloned().collect()
    }
}

/// Concrete error type for platform wrappers
#[derive(Debug, thiserror::Error)]
pub enum PlatformWrapperError {
    #[error("Twitch error: {0}")]
    Twitch(#[from] crate::platforms::twitch::TwitchError),
    #[error("Kick error: {0}")]
    Kick(#[from] crate::platforms::kick::KickError),
    #[error("Generic platform error: {0}")]
    Generic(String),
}

// thiserror already implements std::error::Error, Send, and Sync for PlatformWrapperError
// and the blanket From implementation is already provided by the standard library

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilidades comunes para plataformas
pub mod utils {
    use super::*;

    pub fn sanitize_username(username: &str) -> String {
        username
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase()
    }

    pub fn sanitize_channel_name(channel: &str) -> String {
        channel
            .trim_start_matches('@')
            .trim_start_matches('#')
            .to_lowercase()
    }

    pub fn extract_channel_from_url(url: &str) -> Option<String> {
        if url.contains("twitch.tv/") {
            let parts: Vec<&str> = url.split("twitch.tv/").collect();
            if parts.len() > 1 {
                let channel = parts[1].split('/').next()?;
                return Some(sanitize_channel_name(channel));
            }
        } else if url.contains("youtube.com/") {
            if url.contains("/channel/") {
                let parts: Vec<&str> = url.split("/channel/").collect();
                if parts.len() > 1 {
                    return Some(parts[1].split('/').next()?.to_string());
                }
            } else if url.contains("/c/") {
                let parts: Vec<&str> = url.split("/c/").collect();
                if parts.len() > 1 {
                    return Some(parts[1].split('/').next()?.to_string());
                }
            } else if url.contains("/@") {
                let parts: Vec<&str> = url.split("/@").collect();
                if parts.len() > 1 {
                    return Some(parts[1].split('/').next()?.to_string());
                }
            }
        } else if url.contains("kick.com/") {
            let parts: Vec<&str> = url.split("kick.com/").collect();
            if parts.len() > 1 {
                return Some(parts[1].split('/').next()?.to_string());
            }
        }

        None
    }

    pub fn validate_message_content(
        content: &str,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        // Longitud mínima
        if let Some(min_len) = filters.min_message_length {
            if content.len() < min_len {
                return false;
            }
        }

        // Longitud máxima
        if let Some(max_len) = filters.max_message_length {
            if content.len() > max_len {
                return false;
            }
        }

        // Palabras bloqueadas
        let content_lower = content.to_lowercase();
        for blocked_word in &filters.blocked_words {
            if content_lower.contains(&blocked_word.to_lowercase()) {
                return false;
            }
        }

        true
    }

    pub fn generate_message_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        format!("msg_{}_{}", timestamp, rand::random::<u32>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_username() {
        assert_eq!(utils::sanitize_username("Test_User123"), "test_user123");
        assert_eq!(utils::sanitize_username("@User!"), "user");
    }

    #[test]
    fn test_sanitize_channel_name() {
        assert_eq!(utils::sanitize_channel_name("#channel"), "channel");
        assert_eq!(utils::sanitize_channel_name("@Channel"), "channel");
    }

    #[test]
    fn test_extract_channel_from_url() {
        assert_eq!(
            utils::extract_channel_from_url("https://twitch.tv/streamer"),
            Some("streamer".to_string())
        );
        assert_eq!(
            utils::extract_channel_from_url("https://kick.com/streamer"),
            Some("streamer".to_string())
        );
    }
}
