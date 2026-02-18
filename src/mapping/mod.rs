pub mod data_mapper;
pub mod message_transformer;

pub use data_mapper::*;
pub use message_transformer::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sistema central de mapeo entre diferentes plataformas de streaming
pub struct MappingSystem {
    data_mapper: DataMapper,
    message_transformer: MessageTransformer,
    platform_adapters: HashMap<String, Box<dyn PlatformAdapter>>,
    config: MappingConfig,
}

impl MappingSystem {
    pub fn new(config: MappingConfig) -> Self {
        // Los adaptadores de plataforma ahora se registran dinámicamente
        // desde el exterior cuando se reciben mensajes via transport
        Self {
            data_mapper: DataMapper::new(),
            message_transformer: MessageTransformer::new(),
            platform_adapters: HashMap::new(),
            config,
        }
    }

    /// Mapea un mensaje desde una plataforma a un formato unificado
    pub async fn map_message(
        &mut self,
        raw_message: &RawPlatformMessage,
    ) -> Result<MappedMessage, MappingError> {
        // Obtener adaptador para la plataforma
        let adapter = self
            .platform_adapters
            .get(&raw_message.platform)
            .ok_or_else(|| MappingError::UnsupportedPlatform(raw_message.platform.clone()))?;

        // Transformar mensaje crudo a formato estandarizado
        let standardized = adapter.transform_message(raw_message).await?;

        // Aplicar transformaciones adicionales
        let transformed = self
            .message_transformer
            .transform(standardized, &self.config)?;

        // Mapear datos adicionales
        let mapped = self.data_mapper.map_data(transformed).await?;

        Ok(mapped)
    }

    /// Registra un nuevo adaptador de plataforma
    pub fn register_adapter(&mut self, platform: String, adapter: Box<dyn PlatformAdapter>) {
        self.platform_adapters.insert(platform, adapter);
    }

    /// Obtiene lista de plataformas soportadas
    pub fn supported_platforms(&self) -> Vec<String> {
        self.platform_adapters.keys().cloned().collect()
    }

    /// Actualiza configuración de mapeo
    pub fn update_config(&mut self, config: MappingConfig) {
        self.config = config;
    }
}

impl Default for MappingSystem {
    fn default() -> Self {
        Self::new(MappingConfig::default())
    }
}

/// Mensaje crudo desde cualquier plataforma
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPlatformMessage {
    pub platform: String,
    pub channel: String,
    pub raw_data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message_id: Option<String>,
}

/// Mensaje mapeado y estandarizado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedMessage {
    pub id: String,
    pub platform: String,
    pub channel: String,
    pub username: String,
    pub display_name: Option<String>,
    pub content: String,
    pub emotes: Vec<crate::connection::Emote>,
    pub badges: Vec<crate::connection::Badge>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_level: UserLevel,
    pub message_type: MappedMessageType,
    pub metadata: MappedMetadata,
}

/// Nivel de usuario unificado
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserLevel {
    Normal,
    Subscriber,
    Vip,
    Moderator,
    Broadcaster,
    Staff,
    Admin,
    GlobalModerator,
    Unknown,
}

/// Tipo de mensaje mapeado
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MappedMessageType {
    Normal,
    Action,
    System,
    Whisper,
    Highlight,
    Subscription,
    Raid,
    Cheer,
    Poll,
    Prediction,
    Timeout,
    Ban,
    Unknown,
}

/// Metadatos mapeados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedMetadata {
    pub is_action: bool,
    pub is_whisper: bool,
    pub is_highlighted: bool,
    pub is_me_message: bool,
    pub is_deleted: bool,
    pub reply_to: Option<String>,
    pub thread_id: Option<String>,
    pub cheer_amount: Option<u32>,
    pub subscription_months: Option<u32>,
    pub raid_viewers: Option<u32>,
    pub timeout_duration: Option<u32>,
    pub custom_data: HashMap<String, serde_json::Value>,
}

/// Configuración del sistema de mapeo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub normalize_usernames: bool,
    pub normalize_channels: bool,
    pub convert_timestamps: bool,
    pub filter_system_messages: bool,
    pub merge_duplicate_emotes: bool,
    pub resolve_user_levels: bool,
    pub custom_mappings: HashMap<String, serde_json::Value>,
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self {
            normalize_usernames: true,
            normalize_channels: true,
            convert_timestamps: true,
            filter_system_messages: false,
            merge_duplicate_emotes: true,
            resolve_user_levels: true,
            custom_mappings: HashMap::new(),
        }
    }
}

/// Errores del sistema de mapeo
#[derive(Debug, thiserror::Error)]
pub enum MappingError {
    #[error("Plataforma no soportada: {0}")]
    UnsupportedPlatform(String),

    #[error("Error de transformación: {0}")]
    TransformationError(String),

    #[error("Error de parseo: {0}")]
    ParseError(String),

    #[error("Error de validación: {0}")]
    ValidationError(String),

    #[error("Error de configuración: {0}")]
    ConfigError(String),

    #[error("Error interno: {0}")]
    InternalError(String),
}

/// Trait para adaptadores de plataforma
#[async_trait::async_trait]
pub trait PlatformAdapter: Send + Sync {
    /// Transforma un mensaje crudo a formato estandarizado
    async fn transform_message(
        &self,
        raw_message: &RawPlatformMessage,
    ) -> Result<StandardizedMessage, MappingError>;

    /// Nombre de la plataforma
    fn platform_name(&self) -> &str;

    /// Mapea niveles de usuario específicos de la plataforma
    fn map_user_level(&self, platform_level: &str) -> UserLevel;

    /// Mapea tipos de mensaje específicos de la plataforma
    fn map_message_type(&self, platform_type: &str) -> MappedMessageType;

    /// Extrae emotes del mensaje crudo
    fn extract_emotes(&self, raw_data: &serde_json::Value) -> Vec<crate::connection::Emote>;

    /// Extrae badges del mensaje crudo
    fn extract_badges(&self, raw_data: &serde_json::Value) -> Vec<crate::connection::Badge>;
}

/// Mensaje estandarizado intermedio
#[derive(Debug, Clone)]
pub struct StandardizedMessage {
    pub platform: String,
    pub channel: String,
    pub username: String,
    pub display_name: Option<String>,
    pub content: String,
    pub emotes: Vec<crate::connection::Emote>,
    pub badges: Vec<crate::connection::Badge>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_level: UserLevel,
    pub message_type: MappedMessageType,
    pub raw_data: serde_json::Value,
}

