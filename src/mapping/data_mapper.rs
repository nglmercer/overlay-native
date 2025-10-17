use crate::connection::{Badge, ChatMessage, Emote, EmoteSource};
use crate::mapping::{
    MappedMessage, MappedMessageType, MappedMetadata, StandardizedMessage, UserLevel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Data mapper que convierte datos entre diferentes formatos de plataforma
pub struct DataMapper {
    user_level_mappings: HashMap<String, HashMap<String, UserLevel>>,
    message_type_mappings: HashMap<String, HashMap<String, MappedMessageType>>,
    emote_source_mappings: HashMap<String, EmoteSource>,
    custom_mappings: HashMap<String, serde_json::Value>,
}

impl DataMapper {
    pub fn new() -> Self {
        let mut mapper = Self {
            user_level_mappings: HashMap::new(),
            message_type_mappings: HashMap::new(),
            emote_source_mappings: HashMap::new(),
            custom_mappings: HashMap::new(),
        };

        // Inicializar mapeos por defecto
        mapper.initialize_default_mappings();
        mapper
    }

    fn initialize_default_mappings(&mut self) {
        // Mapeos de niveles de usuario para Twitch
        let mut twitch_user_levels = HashMap::new();
        twitch_user_levels.insert("broadcaster".to_string(), UserLevel::Broadcaster);
        twitch_user_levels.insert("moderator".to_string(), UserLevel::Moderator);
        twitch_user_levels.insert("vip".to_string(), UserLevel::Vip);
        twitch_user_levels.insert("subscriber".to_string(), UserLevel::Subscriber);
        twitch_user_levels.insert("staff".to_string(), UserLevel::Staff);
        twitch_user_levels.insert("admin".to_string(), UserLevel::Admin);
        twitch_user_levels.insert("global_mod".to_string(), UserLevel::GlobalModerator);
        twitch_user_levels.insert("".to_string(), UserLevel::Normal);
        self.user_level_mappings
            .insert("twitch".to_string(), twitch_user_levels);

        // Mapeos de tipos de mensaje para Twitch
        let mut twitch_message_types = HashMap::new();
        twitch_message_types.insert("privmsg".to_string(), MappedMessageType::Normal);
        twitch_message_types.insert("action".to_string(), MappedMessageType::Action);
        twitch_message_types.insert("whisper".to_string(), MappedMessageType::Whisper);
        twitch_message_types.insert("notice".to_string(), MappedMessageType::System);
        twitch_message_types.insert("usernotice".to_string(), MappedMessageType::Subscription);
        twitch_message_types.insert("clearchat".to_string(), MappedMessageType::Timeout);
        twitch_message_types.insert("clearmsg".to_string(), MappedMessageType::Ban);
        self.message_type_mappings
            .insert("twitch".to_string(), twitch_message_types);

        // Mapeos de fuentes de emote
        self.emote_source_mappings
            .insert("twitch".to_string(), EmoteSource::Twitch);
        self.emote_source_mappings
            .insert("bttv".to_string(), EmoteSource::BTTV);
        self.emote_source_mappings
            .insert("ffz".to_string(), EmoteSource::FFZ);
        self.emote_source_mappings
            .insert("7tv".to_string(), EmoteSource::SevenTV);
        self.emote_source_mappings
            .insert("youtube".to_string(), EmoteSource::YouTube);
        self.emote_source_mappings
            .insert("kick".to_string(), EmoteSource::Kick);
    }

    /// Mapea un mensaje estandarizado a un mensaje completamente mapeado
    pub async fn map_data(
        &mut self,
        standardized: StandardizedMessage,
    ) -> Result<MappedMessage, crate::mapping::MappingError> {
        // Aplicar mapeos personalizados si existen
        let processed = self.apply_custom_mappings(&standardized).await?;

        // Crear mensaje mapeado
        let mapped = MappedMessage {
            id: self.generate_message_id(),
            platform: processed.platform.clone(),
            channel: processed.channel.clone(),
            username: processed.username.clone(),
            display_name: processed.display_name.clone(),
            content: processed.content.clone(),
            emotes: self.map_emotes(processed.emotes.clone(), &processed.platform)?,
            badges: self.map_badges(processed.badges.clone(), &processed.platform)?,
            timestamp: processed.timestamp,
            user_level: processed.user_level.clone(),
            message_type: processed.message_type.clone(),
            metadata: MappedMetadata {
                is_action: processed.content.starts_with("/me"),
                is_whisper: processed.message_type == MappedMessageType::Whisper,
                is_highlighted: self.is_highlighted(&processed),
                is_me_message: processed.content.starts_with("/me"),
                is_deleted: false,
                reply_to: self.extract_reply_id(&processed),
                thread_id: self.extract_thread_id(&processed),
                cheer_amount: self.extract_cheer_amount(&processed),
                subscription_months: self.extract_subscription_months(&processed),
                raid_viewers: self.extract_raid_viewers(&processed),
                timeout_duration: self.extract_timeout_duration(&processed),
                custom_data: if processed.raw_data.is_object() {
                    processed
                        .raw_data
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                } else {
                    std::collections::HashMap::new()
                },
            },
        };

        Ok(mapped)
    }

    /// Aplica mapeos personalizados configurados por el usuario
    async fn apply_custom_mappings(
        &self,
        standardized: &StandardizedMessage,
    ) -> Result<StandardizedMessage, crate::mapping::MappingError> {
        let mut result = standardized.clone();

        // Aplicar transformaciones personalizadas
        if let Some(platform_mappings) = self.custom_mappings.get(&standardized.platform) {
            if let Some(transformations) = platform_mappings.get("transformations") {
                if let Some(transform_array) = transformations.as_array() {
                    for transform in transform_array {
                        if let Some(transform_obj) = transform.as_object() {
                            if let Some(field) = transform_obj.get("field").and_then(|v| v.as_str())
                            {
                                if let Some(operation) =
                                    transform_obj.get("operation").and_then(|v| v.as_str())
                                {
                                    self.apply_field_transformation(
                                        &mut result,
                                        field,
                                        operation,
                                        transform_obj,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Aplica una transformación a un campo específico
    fn apply_field_transformation(
        &self,
        message: &mut StandardizedMessage,
        field: &str,
        operation: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), crate::mapping::MappingError> {
        match field {
            "username" => match operation {
                "lowercase" => message.username = message.username.to_lowercase(),
                "uppercase" => message.username = message.username.to_uppercase(),
                "replace" => {
                    if let Some(from) = params.get("from").and_then(|v| v.as_str()) {
                        if let Some(to) = params.get("to").and_then(|v| v.as_str()) {
                            message.username = message.username.replace(from, to);
                        }
                    }
                }
                _ => {
                    return Err(crate::mapping::MappingError::ValidationError(format!(
                        "Unknown operation: {}",
                        operation
                    )))
                }
            },
            "content" => match operation {
                "lowercase" => message.content = message.content.to_lowercase(),
                "uppercase" => message.content = message.content.to_uppercase(),
                "replace" => {
                    if let Some(from) = params.get("from").and_then(|v| v.as_str()) {
                        if let Some(to) = params.get("to").and_then(|v| v.as_str()) {
                            message.content = message.content.replace(from, to);
                        }
                    }
                }
                "filter_words" => {
                    if let Some(words) = params.get("words").and_then(|v| v.as_array()) {
                        for word in words {
                            if let Some(w) = word.as_str() {
                                message.content = message.content.replace(w, "***");
                            }
                        }
                    }
                }
                _ => {
                    return Err(crate::mapping::MappingError::ValidationError(format!(
                        "Unknown operation: {}",
                        operation
                    )))
                }
            },
            _ => {
                return Err(crate::mapping::MappingError::ValidationError(format!(
                    "Unknown field: {}",
                    field
                )))
            }
        }

        Ok(())
    }

    /// Mapea emotes desde el formato de la plataforma al formato unificado
    fn map_emotes(
        &self,
        emotes: Vec<Emote>,
        platform: &str,
    ) -> Result<Vec<Emote>, crate::mapping::MappingError> {
        let mut mapped_emotes = Vec::new();

        for emote in emotes {
            let mapped_emote = Emote {
                id: emote.id,
                name: emote.name,
                source: emote.source, // Ya debería estar mapeado
                positions: emote.positions,
                url: emote.url,
                is_animated: emote.is_animated,
                width: emote.width,
                height: emote.height,
                metadata: emote.metadata,
            };
            mapped_emotes.push(mapped_emote);
        }

        Ok(mapped_emotes)
    }

    /// Mapea badges desde el formato de la plataforma al formato unificado
    fn map_badges(
        &self,
        badges: Vec<Badge>,
        platform: &str,
    ) -> Result<Vec<Badge>, crate::mapping::MappingError> {
        let mut mapped_badges = Vec::new();

        for badge in badges {
            let mapped_badge = Badge {
                id: badge.id,
                name: badge.name,
                version: badge.version,
                url: badge.url,
                title: badge.title,
                source: badge.source, // Ya debería estar mapeado
            };
            mapped_badges.push(mapped_badge);
        }

        Ok(mapped_badges)
    }

    /// Determina si un mensaje debe ser destacado
    fn is_highlighted(&self, message: &StandardizedMessage) -> bool {
        // Mensajes de suscriptores, VIPs, moderadores o el streamer
        matches!(
            message.user_level,
            UserLevel::Subscriber | UserLevel::Vip | UserLevel::Moderator | UserLevel::Broadcaster
        ) ||
        // Mensajes que mencionan al streamer (requeriría configuración)
        message.content.to_lowercase().contains(&message.channel.to_lowercase())
    }

    /// Extrae ID del mensaje respondido
    fn extract_reply_id(&self, message: &StandardizedMessage) -> Option<String> {
        message
            .raw_data
            .get("reply_parent_msg_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extrae ID del hilo de conversación
    fn extract_thread_id(&self, message: &StandardizedMessage) -> Option<String> {
        message
            .raw_data
            .get("thread_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extrae cantidad de bits en un mensaje de cheer
    fn extract_cheer_amount(&self, message: &StandardizedMessage) -> Option<u32> {
        if message.message_type == MappedMessageType::Cheer {
            // Extraer usando regex o parsing del contenido
            self.parse_cheer_amount(&message.content)
        } else {
            None
        }
    }

    /// Parsea cantidad de bits de un mensaje de cheer
    fn parse_cheer_amount(&self, content: &str) -> Option<u32> {
        use regex::Regex;

        let cheer_regex = Regex::new(r"(?i)(\d+)\s*bits?").ok()?;
        if let Some(captures) = cheer_regex.captures(content) {
            captures.get(1)?.as_str().parse().ok()
        } else {
            None
        }
    }

    /// Extrae meses de suscripción
    fn extract_subscription_months(&self, message: &StandardizedMessage) -> Option<u32> {
        message
            .raw_data
            .get("cumulative_months")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    }

    /// Extrae cantidad de viewers en un raid
    fn extract_raid_viewers(&self, message: &StandardizedMessage) -> Option<u32> {
        message
            .raw_data
            .get("viewer_count")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    }

    /// Extrae duración de timeout
    fn extract_timeout_duration(&self, message: &StandardizedMessage) -> Option<u32> {
        message
            .raw_data
            .get("ban_duration")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    }

    /// Genera un ID único para el mensaje mapeado
    fn generate_message_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        format!("mapped_{}_{}", timestamp, rand::random::<u32>())
    }

    /// Registra mapeos personalizados para una plataforma
    pub fn register_custom_mappings(&mut self, platform: String, mappings: serde_json::Value) {
        self.custom_mappings.insert(platform, mappings);
    }

    /// Registra mapeos de niveles de usuario para una plataforma
    pub fn register_user_level_mappings(
        &mut self,
        platform: String,
        mappings: HashMap<String, UserLevel>,
    ) {
        self.user_level_mappings.insert(platform, mappings);
    }

    /// Registra mapeos de tipos de mensaje para una plataforma
    pub fn register_message_type_mappings(
        &mut self,
        platform: String,
        mappings: HashMap<String, MappedMessageType>,
    ) {
        self.message_type_mappings.insert(platform, mappings);
    }

    /// Obtiene estadísticas del mapeo
    pub fn get_stats(&self) -> DataMapperStats {
        DataMapperStats {
            total_platforms: self.user_level_mappings.len(),
            total_custom_mappings: self.custom_mappings.len(),
            platforms_with_mappings: self.user_level_mappings.keys().cloned().collect(),
        }
    }
}

/// Estadísticas del data mapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMapperStats {
    pub total_platforms: usize,
    pub total_custom_mappings: usize,
    pub platforms_with_mappings: Vec<String>,
}

impl Default for DataMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{EmoteMetadata, TextPosition};
    use chrono::Utc;

    fn create_test_standardized_message() -> StandardizedMessage {
        StandardizedMessage {
            platform: "twitch".to_string(),
            channel: "test_channel".to_string(),
            username: "test_user".to_string(),
            display_name: Some("Test User".to_string()),
            content: "Hello world Kappa".to_string(),
            emotes: vec![Emote {
                id: "25".to_string(),
                name: "Kappa".to_string(),
                source: EmoteSource::Twitch,
                positions: vec![TextPosition { start: 12, end: 16 }],
                url: None,
                is_animated: false,
                width: Some(28),
                height: Some(28),
                metadata: EmoteMetadata::default(),
            }],
            badges: vec![],
            timestamp: Utc::now(),
            user_level: UserLevel::Normal,
            message_type: MappedMessageType::Normal,
            raw_data: serde_json::json!({
                "test_field": "test_value"
            }),
        }
    }

    #[tokio::test]
    async fn test_map_data() {
        let mut mapper = DataMapper::new();
        let standardized = create_test_standardized_message();

        let result = mapper.map_data(standardized).await.unwrap();

        assert_eq!(result.platform, "twitch");
        assert_eq!(result.username, "test_user");
        assert_eq!(result.content, "Hello world Kappa");
        assert_eq!(result.user_level, UserLevel::Normal);
        assert_eq!(result.message_type, MappedMessageType::Normal);
    }

    #[test]
    fn test_parse_cheer_amount() {
        let mapper = DataMapper::new();

        assert_eq!(mapper.parse_cheer_amount("cheer100 bits"), Some(100));
        assert_eq!(mapper.parse_cheer_amount("Cheer50bits"), Some(50));
        assert_eq!(mapper.parse_cheer_amount("no bits here"), None);
    }

    #[test]
    fn test_field_transformation() {
        let mapper = DataMapper::new();
        let mut message = create_test_standardized_message();

        let mut params = serde_json::Map::new();
        params.insert(
            "operation".to_string(),
            serde_json::Value::String("lowercase".to_string()),
        );

        let result =
            mapper.apply_field_transformation(&mut message, "username", "lowercase", &params);
        assert!(result.is_ok());
        assert_eq!(message.username, "test_user");
    }

    #[test]
    fn test_get_stats() {
        let mapper = DataMapper::new();
        let stats = mapper.get_stats();

        assert!(stats.total_platforms > 0);
        assert!(stats
            .platforms_with_mappings
            .contains(&"twitch".to_string()));
    }
}
