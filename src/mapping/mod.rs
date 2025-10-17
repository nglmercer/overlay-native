pub mod data_mapper;
pub mod message_transformer;
pub mod platform_adapter;

pub use data_mapper::*;
pub use message_transformer::*;
pub use platform_adapter::*;

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
        let mut adapters: HashMap<String, Box<dyn PlatformAdapter>> = HashMap::new();

        // Registrar adaptadores por defecto
        adapters.insert("twitch".to_string(), Box::new(TwitchAdapter::new()));
        adapters.insert("youtube".to_string(), Box::new(YouTubeAdapter::new()));
        adapters.insert("kick".to_string(), Box::new(KickAdapter::new()));

        Self {
            data_mapper: DataMapper::new(),
            message_transformer: MessageTransformer::new(),
            platform_adapters: adapters,
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

/// Adaptador para Twitch
pub struct TwitchAdapter;

impl TwitchAdapter {
    pub fn new() -> Self {
        Self
    }

    fn extract_user_from_twitch_message(
        &self,
        raw_data: &serde_json::Value,
    ) -> (String, Option<String>, UserLevel) {
        // Extraer información del usuario desde datos de Twitch
        if let Some(user) = raw_data.get("user") {
            let username = user
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let display_name = user
                .get("display_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let user_level = if let Some(badges) = user.get("badges") {
                if let Some(badges_array) = badges.as_array() {
                    for badge in badges_array {
                        if let Some(badge_id) = badge.get("id").and_then(|v| v.as_str()) {
                            match badge_id {
                                "broadcaster" => {
                                    return (username, display_name, UserLevel::Broadcaster)
                                }
                                "moderator" => {
                                    return (username, display_name, UserLevel::Moderator)
                                }
                                "vip" => return (username, display_name, UserLevel::Vip),
                                "subscriber" => {
                                    return (username, display_name, UserLevel::Subscriber)
                                }
                                "staff" => return (username, display_name, UserLevel::Staff),
                                "admin" => return (username, display_name, UserLevel::Admin),
                                "global_mod" => {
                                    return (username, display_name, UserLevel::GlobalModerator)
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                UserLevel::Normal
            } else {
                UserLevel::Normal
            };

            (username, display_name, user_level)
        } else {
            ("unknown".to_string(), None, UserLevel::Normal)
        }
    }

    fn extract_emotes_from_twitch(
        &self,
        raw_data: &serde_json::Value,
    ) -> Vec<crate::connection::Emote> {
        let mut emotes = Vec::new();

        if let Some(emotes_data) = raw_data.get("emotes") {
            if let Some(emotes_array) = emotes_data.as_array() {
                for emote in emotes_array {
                    if let (Some(id), Some(name)) = (
                        emote.get("id").and_then(|v| v.as_str()),
                        emote.get("name").and_then(|v| v.as_str()),
                    ) {
                        let positions = if let Some(pos_array) =
                            emote.get("positions").and_then(|v| v.as_array())
                        {
                            pos_array
                                .iter()
                                .filter_map(|pos| {
                                    if let (Some(start), Some(end)) = (
                                        pos.get("start").and_then(|v| v.as_u64()),
                                        pos.get("end").and_then(|v| v.as_u64()),
                                    ) {
                                        Some(crate::connection::TextPosition {
                                            start: start as usize,
                                            end: end as usize,
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };

                        emotes.push(crate::connection::Emote {
                            id: id.to_string(),
                            name: name.to_string(),
                            source: crate::connection::EmoteSource::Twitch,
                            positions,
                            url: None,
                            is_animated: false,
                            width: Some(28),
                            height: Some(28),
                            metadata: crate::connection::EmoteMetadata {
                                is_zero_width: false,
                                modifier: false,
                                emote_set_id: Some(id.to_string()),
                                tier: None,
                            },
                        });
                    }
                }
            }
        }

        emotes
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for TwitchAdapter {
    async fn transform_message(
        &self,
        raw_message: &RawPlatformMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        let (username, display_name, user_level) =
            self.extract_user_from_twitch_message(&raw_message.raw_data);

        let content = raw_message
            .raw_data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let emotes = self.extract_emotes_from_twitch(&raw_message.raw_data);
        let badges = self.extract_badges(&raw_message.raw_data);

        let message_type =
            if let Some(msg_type) = raw_message.raw_data.get("type").and_then(|v| v.as_str()) {
                self.map_message_type(msg_type)
            } else {
                MappedMessageType::Normal
            };

        Ok(StandardizedMessage {
            platform: raw_message.platform.clone(),
            channel: raw_message.channel.clone(),
            username,
            display_name,
            content,
            emotes,
            badges,
            timestamp: raw_message.timestamp,
            user_level,
            message_type,
            raw_data: raw_message.raw_data.clone(),
        })
    }

    fn platform_name(&self) -> &str {
        "twitch"
    }

    fn map_user_level(&self, platform_level: &str) -> UserLevel {
        match platform_level.to_lowercase().as_str() {
            "broadcaster" => UserLevel::Broadcaster,
            "moderator" => UserLevel::Moderator,
            "vip" => UserLevel::Vip,
            "subscriber" => UserLevel::Subscriber,
            "staff" => UserLevel::Staff,
            "admin" => UserLevel::Admin,
            "global_mod" => UserLevel::GlobalModerator,
            _ => UserLevel::Normal,
        }
    }

    fn map_message_type(&self, platform_type: &str) -> MappedMessageType {
        match platform_type.to_lowercase().as_str() {
            "privmsg" => MappedMessageType::Normal,
            "action" => MappedMessageType::Action,
            "whisper" => MappedMessageType::Whisper,
            "notice" => MappedMessageType::System,
            "usernotice" => MappedMessageType::Subscription,
            "clearchat" => MappedMessageType::Timeout,
            "clearmsg" => MappedMessageType::Ban,
            _ => MappedMessageType::Unknown,
        }
    }

    fn extract_emotes(&self, raw_data: &serde_json::Value) -> Vec<crate::connection::Emote> {
        self.extract_emotes_from_twitch(raw_data)
    }

    fn extract_badges(&self, raw_data: &serde_json::Value) -> Vec<crate::connection::Badge> {
        let mut badges = Vec::new();

        if let Some(badges_data) = raw_data.get("badges") {
            if let Some(badges_array) = badges_data.as_array() {
                for badge in badges_array {
                    if let (Some(id), Some(version)) = (
                        badge.get("id").and_then(|v| v.as_str()),
                        badge.get("version").and_then(|v| v.as_str()),
                    ) {
                        badges.push(crate::connection::Badge {
                            id: id.to_string(),
                            name: id.to_string(),
                            version: version.to_string(),
                            url: Some(format!(
                                "https://static-cdn.jtvnw.net/badges/v1/{}/{}",
                                id, version
                            )),
                            title: None,
                            source: crate::connection::EmoteSource::Twitch,
                        });
                    }
                }
            }
        }

        badges
    }
}

impl Default for TwitchAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptador para YouTube (placeholder)
pub struct YouTubeAdapter;

impl YouTubeAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for YouTubeAdapter {
    async fn transform_message(
        &self,
        raw_message: &RawPlatformMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        // Implementación básica para YouTube
        Ok(StandardizedMessage {
            platform: raw_message.platform.clone(),
            channel: raw_message.channel.clone(),
            username: "youtube_user".to_string(),
            display_name: Some("YouTube User".to_string()),
            content: "YouTube message".to_string(),
            emotes: Vec::new(),
            badges: Vec::new(),
            timestamp: raw_message.timestamp,
            user_level: UserLevel::Normal,
            message_type: MappedMessageType::Normal,
            raw_data: raw_message.raw_data.clone(),
        })
    }

    fn platform_name(&self) -> &str {
        "youtube"
    }

    fn map_user_level(&self, _platform_level: &str) -> UserLevel {
        UserLevel::Normal
    }

    fn map_message_type(&self, _platform_type: &str) -> MappedMessageType {
        MappedMessageType::Normal
    }

    fn extract_emotes(&self, _raw_data: &serde_json::Value) -> Vec<crate::connection::Emote> {
        Vec::new()
    }

    fn extract_badges(&self, _raw_data: &serde_json::Value) -> Vec<crate::connection::Badge> {
        Vec::new()
    }
}

impl Default for YouTubeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptador para Kick (placeholder)
pub struct KickAdapter;

impl KickAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl PlatformAdapter for KickAdapter {
    async fn transform_message(
        &self,
        raw_message: &RawPlatformMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        // Implementación básica para Kick
        Ok(StandardizedMessage {
            platform: raw_message.platform.clone(),
            channel: raw_message.channel.clone(),
            username: "kick_user".to_string(),
            display_name: Some("Kick User".to_string()),
            content: "Kick message".to_string(),
            emotes: Vec::new(),
            badges: Vec::new(),
            timestamp: raw_message.timestamp,
            user_level: UserLevel::Normal,
            message_type: MappedMessageType::Normal,
            raw_data: raw_message.raw_data.clone(),
        })
    }

    fn platform_name(&self) -> &str {
        "kick"
    }

    fn map_user_level(&self, _platform_level: &str) -> UserLevel {
        UserLevel::Normal
    }

    fn map_message_type(&self, _platform_type: &str) -> MappedMessageType {
        MappedMessageType::Normal
    }

    fn extract_emotes(&self, _raw_data: &serde_json::Value) -> Vec<crate::connection::Emote> {
        Vec::new()
    }

    fn extract_badges(&self, _raw_data: &serde_json::Value) -> Vec<crate::connection::Badge> {
        Vec::new()
    }
}

impl Default for KickAdapter {
    fn default() -> Self {
        Self::new()
    }
}
