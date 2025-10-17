use crate::mapping::{MappedMessage, MappingConfig, MappingError, StandardizedMessage};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transformer que aplica transformaciones a mensajes estandarizados
pub struct MessageTransformer {
    transformers: Vec<Box<dyn MessageTransform>>,
    config: MappingConfig,
    regex_cache: HashMap<String, Regex>,
}

impl MessageTransformer {
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
            config: MappingConfig::default(),
            regex_cache: HashMap::new(),
        }
    }

    /// Aplica todas las transformaciones configuradas a un mensaje
    pub fn transform(
        &mut self,
        message: StandardizedMessage,
        config: &MappingConfig,
    ) -> Result<StandardizedMessage, MappingError> {
        let mut result = message;

        // Normalizar nombres de usuario si está configurado
        if config.normalize_usernames {
            result = self.normalize_username(result)?;
        }

        // Normalizar nombres de canal si está configurado
        if config.normalize_channels {
            result = self.normalize_channel(result)?;
        }

        // Convertir timestamps si está configurado
        if config.convert_timestamps {
            result = self.convert_timestamp(result)?;
        }

        // Filtrar mensajes de sistema si está configurado
        if config.filter_system_messages && self.is_system_message(&result) {
            return Err(MappingError::ValidationError(
                "System message filtered out".to_string(),
            ));
        }

        // Aplicar transformaciones personalizadas
        result = self.apply_custom_transformations(result, config)?;

        Ok(result)
    }

    /// Normaliza el nombre de usuario
    fn normalize_username(
        &self,
        mut message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        message.username = message.username.to_lowercase();
        if let Some(display_name) = &mut message.display_name {
            *display_name = display_name.to_lowercase();
        }
        Ok(message)
    }

    /// Normaliza el nombre del canal
    fn normalize_channel(
        &self,
        mut message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        message.channel = message.channel.to_lowercase();
        Ok(message)
    }

    /// Convierte timestamps a formato UTC
    fn convert_timestamp(
        &self,
        mut message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        // Asumimos que el timestamp ya está en UTC, pero podríamos hacer conversiones
        Ok(message)
    }

    /// Verifica si un mensaje es de sistema
    fn is_system_message(&self, message: &StandardizedMessage) -> bool {
        matches!(
            message.message_type,
            crate::mapping::MappedMessageType::System
        ) || message.username.to_lowercase() == "system"
    }

    /// Aplica transformaciones personalizadas desde la configuración
    fn apply_custom_transformations(
        &mut self,
        mut message: StandardizedMessage,
        config: &MappingConfig,
    ) -> Result<StandardizedMessage, MappingError> {
        for (key, value) in &config.custom_mappings {
            match key.as_str() {
                "content_transforms" => {
                    if let Some(transforms) = value.as_array() {
                        for transform in transforms {
                            message = self.apply_content_transform(message, transform)?;
                        }
                    }
                }
                "user_transforms" => {
                    if let Some(transforms) = value.as_array() {
                        for transform in transforms {
                            message = self.apply_user_transform(message, transform)?;
                        }
                    }
                }
                "emote_transforms" => {
                    if let Some(transforms) = value.as_array() {
                        for transform in transforms {
                            message = self.apply_emote_transforms(message, transform)?;
                        }
                    }
                }
                _ => {
                    // Transformación genérica
                    message = self.apply_generic_transform(message, key, value)?;
                }
            }
        }

        Ok(message)
    }

    /// Aplica una transformación al contenido del mensaje
    fn apply_content_transform(
        &mut self,
        mut message: StandardizedMessage,
        transform: &serde_json::Value,
    ) -> Result<StandardizedMessage, MappingError> {
        if let Some(transform_obj) = transform.as_object() {
            if let Some(transform_type) = transform_obj.get("type").and_then(|v| v.as_str()) {
                match transform_type {
                    "replace" => {
                        if let Some(from) = transform_obj.get("from").and_then(|v| v.as_str()) {
                            if let Some(to) = transform_obj.get("to").and_then(|v| v.as_str()) {
                                message.content = message.content.replace(from, to);
                            }
                        }
                    }
                    "regex_replace" => {
                        if let Some(pattern) = transform_obj.get("pattern").and_then(|v| v.as_str())
                        {
                            if let Some(replacement) =
                                transform_obj.get("replacement").and_then(|v| v.as_str())
                            {
                                let regex = self.get_or_compile_regex(pattern)?;
                                message.content =
                                    regex.replace_all(&message.content, replacement).to_string();
                            }
                        }
                    }
                    "filter_words" => {
                        if let Some(words) = transform_obj.get("words").and_then(|v| v.as_array()) {
                            for word in words {
                                if let Some(w) = word.as_str() {
                                    let replacement = transform_obj
                                        .get("replacement")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("***");
                                    message.content = message.content.replace(w, replacement);
                                }
                            }
                        }
                    }
                    "case_transform" => {
                        if let Some(case_type) = transform_obj.get("case").and_then(|v| v.as_str())
                        {
                            match case_type {
                                "upper" => message.content = message.content.to_uppercase(),
                                "lower" => message.content = message.content.to_lowercase(),
                                "title" => {
                                    message.content = message
                                        .content
                                        .chars()
                                        .enumerate()
                                        .map(|(i, c)| {
                                            if i == 0
                                                || message.content.chars().nth(i - 1) == Some(' ')
                                            {
                                                c.to_uppercase().to_string()
                                            } else {
                                                c.to_lowercase().to_string()
                                            }
                                        })
                                        .collect();
                                }
                                _ => {}
                            }
                        }
                    }
                    "prepend" => {
                        if let Some(prefix) = transform_obj.get("prefix").and_then(|v| v.as_str()) {
                            message.content = format!("{} {}", prefix, message.content);
                        }
                    }
                    "append" => {
                        if let Some(suffix) = transform_obj.get("suffix").and_then(|v| v.as_str()) {
                            message.content = format!("{} {}", message.content, suffix);
                        }
                    }
                    _ => {
                        return Err(MappingError::ValidationError(format!(
                            "Unknown content transform type: {}",
                            transform_type
                        )));
                    }
                }
            }
        }

        Ok(message)
    }

    /// Aplica una transformación al usuario
    fn apply_user_transform(
        &mut self,
        mut message: StandardizedMessage,
        transform: &serde_json::Value,
    ) -> Result<StandardizedMessage, MappingError> {
        if let Some(transform_obj) = transform.as_object() {
            if let Some(transform_type) = transform_obj.get("type").and_then(|v| v.as_str()) {
                match transform_type {
                    "replace" => {
                        if let Some(from) = transform_obj.get("from").and_then(|v| v.as_str()) {
                            if let Some(to) = transform_obj.get("to").and_then(|v| v.as_str()) {
                                message.username = message.username.replace(from, to);
                                if let Some(display_name) = &mut message.display_name {
                                    *display_name = display_name.replace(from, to);
                                }
                            }
                        }
                    }
                    "prefix" => {
                        if let Some(prefix) = transform_obj.get("prefix").and_then(|v| v.as_str()) {
                            message.username = format!("{}{}", prefix, message.username);
                            if let Some(display_name) = &mut message.display_name {
                                *display_name = format!("{}{}", prefix, display_name);
                            }
                        }
                    }
                    "suffix" => {
                        if let Some(suffix) = transform_obj.get("suffix").and_then(|v| v.as_str()) {
                            message.username = format!("{}{}", message.username, suffix);
                            if let Some(display_name) = &mut message.display_name {
                                *display_name = format!("{}{}", display_name, suffix);
                            }
                        }
                    }
                    "anonimize" => {
                        message.username = self.anonymize_username(&message.username);
                        if let Some(display_name) = &mut message.display_name {
                            *display_name = self.anonymize_username(display_name);
                        }
                    }
                    _ => {
                        return Err(MappingError::ValidationError(format!(
                            "Unknown user transform type: {}",
                            transform_type
                        )));
                    }
                }
            }
        }

        Ok(message)
    }

    /// Aplica transformaciones a emotes
    fn apply_emote_transforms(
        &mut self,
        mut message: StandardizedMessage,
        transform: &serde_json::Value,
    ) -> Result<StandardizedMessage, MappingError> {
        if let Some(transform_obj) = transform.as_object() {
            if let Some(transform_type) = transform_obj.get("type").and_then(|v| v.as_str()) {
                match transform_type {
                    "filter" => {
                        if let Some(emotes_to_filter) =
                            transform_obj.get("emotes").and_then(|v| v.as_array())
                        {
                            let filter_list: Vec<String> = emotes_to_filter
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect();

                            message
                                .emotes
                                .retain(|emote| !filter_list.contains(&emote.name));
                        }
                    }
                    "replace" => {
                        if let Some(from) = transform_obj.get("from").and_then(|v| v.as_str()) {
                            if let Some(to) = transform_obj.get("to").and_then(|v| v.as_str()) {
                                for emote in &mut message.emotes {
                                    if emote.name == from {
                                        emote.name = to.to_string();
                                    }
                                }
                            }
                        }
                    }
                    "scale" => {
                        if let Some(scale_factor) =
                            transform_obj.get("scale").and_then(|v| v.as_f64())
                        {
                            for emote in &mut message.emotes {
                                if let (Some(width), Some(height)) = (emote.width, emote.height) {
                                    emote.width = Some((width as f64 * scale_factor) as u32);
                                    emote.height = Some((height as f64 * scale_factor) as u32);
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(MappingError::ValidationError(format!(
                            "Unknown emote transform type: {}",
                            transform_type
                        )));
                    }
                }
            }
        }

        Ok(message)
    }

    /// Aplica una transformación genérica
    fn apply_generic_transform(
        &mut self,
        mut message: StandardizedMessage,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<StandardizedMessage, MappingError> {
        match key {
            "max_message_length" => {
                if let Some(max_len) = value.as_u64() {
                    if message.content.len() > max_len as usize {
                        message.content.truncate(max_len as usize);
                        message.content.push_str("...");
                    }
                }
            }
            "min_message_length" => {
                if let Some(min_len) = value.as_u64() {
                    if message.content.len() < min_len as usize {
                        return Err(MappingError::ValidationError(
                            "Message too short".to_string(),
                        ));
                    }
                }
            }
            "required_user_level" => {
                if let Some(required_level) = value.as_str() {
                    let required = match required_level {
                        "normal" => crate::mapping::UserLevel::Normal,
                        "subscriber" => crate::mapping::UserLevel::Subscriber,
                        "vip" => crate::mapping::UserLevel::Vip,
                        "moderator" => crate::mapping::UserLevel::Moderator,
                        "broadcaster" => crate::mapping::UserLevel::Broadcaster,
                        _ => {
                            return Err(MappingError::ValidationError(
                                "Invalid user level".to_string(),
                            ))
                        }
                    };

                    if !self.user_level_satisfies(&message.user_level, &required) {
                        return Err(MappingError::ValidationError(
                            "Insufficient user level".to_string(),
                        ));
                    }
                }
            }
            _ => {}
        }

        Ok(message)
    }

    /// Anonimiza un nombre de usuario
    fn anonymize_username(&self, username: &str) -> String {
        if username.len() <= 2 {
            "*".repeat(username.len())
        } else {
            format!("{}{}*", &username[..1], "*".repeat(username.len() - 2))
        }
    }

    /// Verifica si un nivel de usuario satisface un requerimiento
    fn user_level_satisfies(
        &self,
        current: &crate::mapping::UserLevel,
        required: &crate::mapping::UserLevel,
    ) -> bool {
        use crate::mapping::UserLevel;

        match (current, required) {
            (_, UserLevel::Normal) => true,
            (
                UserLevel::Subscriber
                | UserLevel::Vip
                | UserLevel::Moderator
                | UserLevel::Broadcaster
                | UserLevel::Staff
                | UserLevel::Admin
                | UserLevel::GlobalModerator,
                UserLevel::Subscriber,
            ) => true,
            (
                UserLevel::Vip
                | UserLevel::Moderator
                | UserLevel::Broadcaster
                | UserLevel::Staff
                | UserLevel::Admin
                | UserLevel::GlobalModerator,
                UserLevel::Vip,
            ) => true,
            (
                UserLevel::Moderator
                | UserLevel::Broadcaster
                | UserLevel::Staff
                | UserLevel::Admin
                | UserLevel::GlobalModerator,
                UserLevel::Moderator,
            ) => true,
            (
                UserLevel::Broadcaster
                | UserLevel::Staff
                | UserLevel::Admin
                | UserLevel::GlobalModerator,
                UserLevel::Broadcaster,
            ) => true,
            (
                UserLevel::Staff | UserLevel::Admin | UserLevel::GlobalModerator,
                UserLevel::Staff,
            ) => true,
            (UserLevel::Admin | UserLevel::GlobalModerator, UserLevel::Admin) => true,
            (UserLevel::GlobalModerator, UserLevel::GlobalModerator) => true,
            _ => false,
        }
    }

    /// Obtiene o compila una expresión regex (con cache)
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex, MappingError> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)
                .map_err(|e| MappingError::ParseError(format!("Invalid regex: {}", e)))?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        // This unwrap is safe because we ensure the key exists above.
        Ok(self.regex_cache.get(pattern).unwrap())
    }

    /// Registra un transformer personalizado
    pub fn register_transformer(&mut self, transformer: Box<dyn MessageTransform>) {
        self.transformers.push(transformer);
    }

    /// Aplica todos los transformers registrados
    pub fn apply_registered_transformers(
        &mut self,
        mut message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        for transformer in &mut self.transformers {
            message = transformer.transform(message)?;
        }
        Ok(message)
    }
}

/// Trait para transformaciones personalizadas de mensajes
pub trait MessageTransform: Send + Sync {
    fn transform(
        &mut self,
        message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError>;
    fn name(&self) -> &str;
}

/// Transformer que filtra mensajes por contenido
pub struct ContentFilter {
    blocked_words: Vec<String>,
    case_sensitive: bool,
}

impl ContentFilter {
    pub fn new(blocked_words: Vec<String>, case_sensitive: bool) -> Self {
        Self {
            blocked_words,
            case_sensitive,
        }
    }
}

impl MessageTransform for ContentFilter {
    fn transform(
        &mut self,
        message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        let content = if self.case_sensitive {
            message.content.clone()
        } else {
            message.content.to_lowercase()
        };

        for word in &self.blocked_words {
            let check_word = if self.case_sensitive {
                word.clone()
            } else {
                word.to_lowercase()
            };

            if content.contains(&check_word) {
                return Err(MappingError::ValidationError(
                    "Message contains blocked word".to_string(),
                ));
            }
        }

        Ok(message)
    }

    fn name(&self) -> &str {
        "content_filter"
    }
}

/// Transformer que añade prefijos/sufijos basados en nivel de usuario
pub struct UserLevelPrefix {
    prefixes: HashMap<crate::mapping::UserLevel, String>,
}

impl UserLevelPrefix {
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert(crate::mapping::UserLevel::Broadcaster, "[👑] ".to_string());
        prefixes.insert(crate::mapping::UserLevel::Moderator, "[🔧] ".to_string());
        prefixes.insert(crate::mapping::UserLevel::Vip, "[💎] ".to_string());
        prefixes.insert(crate::mapping::UserLevel::Subscriber, "[⭐] ".to_string());

        Self { prefixes }
    }

    pub fn with_prefixes(prefixes: HashMap<crate::mapping::UserLevel, String>) -> Self {
        Self { prefixes }
    }
}

impl MessageTransform for UserLevelPrefix {
    fn transform(
        &mut self,
        mut message: StandardizedMessage,
    ) -> Result<StandardizedMessage, MappingError> {
        if let Some(prefix) = self.prefixes.get(&message.user_level) {
            message.username = format!("{}{}", prefix, message.username);
        }
        Ok(message)
    }

    fn name(&self) -> &str {
        "user_level_prefix"
    }
}

impl Default for MessageTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{Emote, EmoteMetadata, EmoteSource, TextPosition};
    use crate::mapping::{MappedMessageType, UserLevel};
    use chrono::Utc;

    fn create_test_message() -> StandardizedMessage {
        StandardizedMessage {
            platform: "twitch".to_string(),
            channel: "test_channel".to_string(),
            username: "TestUser".to_string(),
            display_name: Some("TestUser".to_string()),
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
            raw_data: serde_json::json!({}),
        }
    }

    #[test]
    fn test_normalize_username() {
        let transformer = MessageTransformer::new();
        let message = create_test_message();
        let result = transformer.normalize_username(message).unwrap();
        assert_eq!(result.username, "testuser");
        assert_eq!(result.display_name, Some("testuser".to_string()));
    }

    #[test]
    fn test_anonymize_username() {
        let transformer = MessageTransformer::new();
        assert_eq!(transformer.anonymize_username("username"), "u******");
        assert_eq!(transformer.anonymize_username("ab"), "**");
        assert_eq!(transformer.anonymize_username("a"), "*");
    }

    #[test]
    fn test_content_filter() {
        let mut filter = ContentFilter::new(vec!["spam".to_string()], false);
        let message = create_test_message();

        // Mensaje sin palabras bloqueadas
        let result = filter.transform(message).unwrap();
        assert_eq!(result.content, "Hello world Kappa");

        // Mensaje con palabra bloqueada
        let mut blocked_message = create_test_message();
        blocked_message.content = "This is spam".to_string();
        let result = filter.transform(blocked_message);
        assert!(result.is_err());
    }

    #[test]
    fn test_user_level_prefix() {
        let mut prefix_transform = UserLevelPrefix::new();
        let mut message = create_test_message();
        message.user_level = UserLevel::Moderator;

        let result = prefix_transform.transform(message).unwrap();
        assert_eq!(result.username, "[🔧] TestUser");
    }

    #[test]
    fn test_user_level_satisfaction() {
        let transformer = MessageTransformer::new();

        assert!(transformer.user_level_satisfies(&UserLevel::Moderator, &UserLevel::Normal));
        assert!(transformer.user_level_satisfies(&UserLevel::Moderator, &UserLevel::Subscriber));
        assert!(transformer.user_level_satisfies(&UserLevel::Moderator, &UserLevel::Vip));
        assert!(transformer.user_level_satisfies(&UserLevel::Moderator, &UserLevel::Moderator));
        assert!(!transformer.user_level_satisfies(&UserLevel::Normal, &UserLevel::Moderator));
        assert!(!transformer.user_level_satisfies(&UserLevel::Subscriber, &UserLevel::Moderator));
    }
}
