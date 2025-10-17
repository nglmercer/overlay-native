use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use tokio::sync::{mpsc, RwLock};

use crate::config::{Credentials, PlatformConfig, PlatformSettings};
use crate::connection::{
    Badge, ChatMessage, Emote, EmoteMetadata, EmoteSource, MessageMetadata, MessageType,
    StreamingPlatform, TextPosition,
};

/// Estructura base para todas las plataformas de streaming
pub struct BasePlatform {
    pub platform_name: String,
    pub platform_type: crate::config::PlatformType,
    pub config: PlatformConfig,
    pub connected: bool,
    pub channels: HashMap<String, ChannelInfo>,
    pub credentials: Credentials,
    pub settings: PlatformSettings,
    pub message_queue: mpsc::UnboundedReceiver<ChatMessage>,
    pub message_sender: mpsc::UnboundedSender<ChatMessage>,
    pub emote_cache: crate::connection::EmoteCache,
    pub rate_limiter: RateLimiter,
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub joined_at: Instant,
    pub message_count: u64,
    pub viewer_count: Option<u32>,
    pub live: bool,
    pub title: Option<String>,
    pub category: Option<String>,
}

/// Limitador de tasa para evitar spam
#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub max_messages_per_second: u32,
    pub max_messages_per_minute: u32,
    pub current_second: u32,
    pub current_minute: u32,
    pub last_second_reset: Instant,
    pub last_minute_reset: Instant,
}

impl RateLimiter {
    pub fn new(max_per_second: u32, max_per_minute: u32) -> Self {
        Self {
            max_messages_per_second: max_per_second,
            max_messages_per_minute: max_per_minute,
            current_second: 0,
            current_minute: 0,
            last_second_reset: Instant::now(),
            last_minute_reset: Instant::now(),
        }
    }

    pub fn can_send_message(&mut self) -> bool {
        let now = Instant::now();

        // Resetear contadores si es necesario
        if now.duration_since(self.last_second_reset).as_secs() >= 1 {
            self.current_second = 0;
            self.last_second_reset = now;
        }

        if now.duration_since(self.last_minute_reset).as_secs() >= 60 {
            self.current_minute = 0;
            self.last_minute_reset = now;
        }

        self.current_second < self.max_messages_per_second
            && self.current_minute < self.max_messages_per_minute
    }

    pub fn record_message(&mut self) {
        self.current_second += 1;
        self.current_minute += 1;
    }
}

impl BasePlatform {
    pub fn new(
        platform_name: String,
        platform_type: crate::config::PlatformType,
        config: PlatformConfig,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            platform_name,
            platform_type,
            settings: config.settings.clone(),
            credentials: config.credentials.clone(),
            connected: false,
            channels: HashMap::new(),
            config,
            message_queue: receiver,
            message_sender: sender,
            emote_cache: crate::connection::EmoteCache::new(24),
            rate_limiter: RateLimiter::new(20, 100), // Valores por defecto
        }
    }

    /// Parsea emotes genéricos basados en el formato de datos
    pub fn parse_generic_emotes(&self, content: &str, emote_data: &str) -> Vec<Emote> {
        let mut emotes = Vec::new();

        // Implementación genérica de parsing de emotes
        // Este método puede ser sobreescrito por cada plataforma
        if emote_data.is_empty() {
            return emotes;
        }

        // Formato genérico: "emote_id:start-end,start-end/..."
        for emote_part in emote_data.split('/') {
            let parts: Vec<&str> = emote_part.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let emote_id = parts[0];
            let positions = parts[1];

            for position in positions.split(',') {
                let pos_parts: Vec<&str> = position.split('-').collect();
                if pos_parts.len() != 2 {
                    continue;
                }

                if let (Ok(start), Ok(end)) =
                    (pos_parts[0].parse::<usize>(), pos_parts[1].parse::<usize>())
                {
                    if start < content.len() && end <= content.len() {
                        let emote_name = content[start..=end].to_string();

                        emotes.push(Emote {
                            id: emote_id.to_string(),
                            name: emote_name,
                            source: self.get_default_emote_source(),
                            positions: vec![TextPosition { start, end }],
                            url: None,
                            is_animated: false,
                            width: None,
                            height: None,
                            metadata: EmoteMetadata {
                                is_zero_width: false,
                                modifier: false,
                                emote_set_id: None,
                                tier: None,
                            },
                        });
                    }
                }
            }
        }

        emotes
    }

    /// Parsea badges genéricos
    pub fn parse_generic_badges(&self, badge_data: &str) -> Vec<Badge> {
        let mut badges = Vec::new();

        if badge_data.is_empty() {
            return badges;
        }

        // Formato genérico: "badge1/version1,badge2/version2"
        for badge_part in badge_data.split(',') {
            let parts: Vec<&str> = badge_part.split('/').collect();
            if parts.len() != 2 {
                continue;
            }

            badges.push(Badge {
                id: parts[0].to_string(),
                name: parts[0].to_string(),
                version: parts[1].to_string(),
                url: None,
                title: None,
                source: self.get_default_emote_source(),
            });
        }

        badges
    }

    /// Crea un mensaje de chat base
    pub fn create_base_message(
        &self,
        username: String,
        content: String,
        channel: String,
        message_type: MessageType,
    ) -> ChatMessage {
        ChatMessage {
            id: crate::platforms::utils::generate_message_id(),
            platform: self.platform_name.clone(),
            channel,
            username: username.clone(),
            display_name: Some(username.clone()),
            content,
            emotes: Vec::new(),
            badges: Vec::new(),
            timestamp: SystemTime::now(),
            user_color: None,
            message_type,
            metadata: MessageMetadata {
                is_action: false,
                is_whisper: false,
                is_highlighted: false,
                is_me_message: false,
                reply_to: None,
                thread_id: None,
                custom_data: HashMap::new(),
            },
        }
    }

    /// Obtiene el source de emote por defecto para esta plataforma
    pub fn get_default_emote_source(&self) -> EmoteSource {
        match self.platform_type {
            crate::config::PlatformType::Twitch => EmoteSource::Twitch,
            crate::config::PlatformType::YouTube => EmoteSource::YouTube,
            crate::config::PlatformType::Kick => EmoteSource::Kick,
            crate::config::PlatformType::Trovo => EmoteSource::Trovo,
            crate::config::PlatformType::Facebook => EmoteSource::Facebook,
        }
    }

    /// Actualiza información del canal
    pub fn update_channel_info(&mut self, channel: String, info: ChannelInfo) {
        self.channels.insert(channel.clone(), info);
    }

    /// Obtiene información de un canal
    pub fn get_channel_info(&self, channel: &str) -> Option<&ChannelInfo> {
        self.channels.get(channel)
    }

    /// Verifica si está unido a un canal
    pub fn is_in_channel(&self, channel: &str) -> bool {
        self.channels.contains_key(channel)
    }

    /// Obtiene lista de canales activos
    pub fn get_active_channels(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
    }

    /// Aplica filtros de mensaje
    pub fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        // Verificar longitud del mensaje
        if let Some(min_len) = filters.min_message_length {
            if message.content.len() < min_len {
                return false;
            }
        }

        if let Some(max_len) = filters.max_message_length {
            if message.content.len() > max_len {
                return false;
            }
        }

        // Verificar usuarios bloqueados
        if filters
            .blocked_users
            .contains(&message.username.to_lowercase())
        {
            return false;
        }

        // Verificar lista blanca (si existe)
        if !filters.allowed_users.is_empty()
            && !filters
                .allowed_users
                .contains(&message.username.to_lowercase())
        {
            return false;
        }

        // Verificar palabras bloqueadas
        let content_lower = message.content.to_lowercase();
        for blocked_word in &filters.blocked_words {
            if content_lower.contains(&blocked_word.to_lowercase()) {
                return false;
            }
        }

        // Verificar si es comando
        if filters.commands_only
            && !message.content.starts_with('!')
            && !message.content.starts_with('/')
        {
            return false;
        }

        true
    }

    /// Maneja reconexión automática
    pub async fn handle_reconnect(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.connected {
            return Ok(());
        }

        for attempt in 1..=self.settings.max_reconnect_attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.settings.reconnect_delay_ms * attempt as u64,
            ))
            .await;

            // Note: Reconnection logic should be handled by the implementing platform
            // This method is kept for backward compatibility but should be overridden
            return Err("BasePlatform cannot handle reconnection directly. Implement this in the specific platform.".into());
        }

        Ok(())
    }
}

impl Default for BasePlatform {
    fn default() -> Self {
        Self::new(
            "unknown".to_string(),
            crate::config::PlatformType::Twitch,
            PlatformConfig::default(),
        )
    }
}

/// Utilidades para manejo de emotes multiplataforma
pub mod emote_utils {
    use super::*;

    /// Convierte emotes de diferentes plataformas a un formato unificado
    pub fn normalize_emotes(platform_emotes: Vec<RawEmote>, source: EmoteSource) -> Vec<Emote> {
        platform_emotes
            .into_iter()
            .map(|raw| Emote {
                id: raw.id,
                name: raw.name,
                source: source.clone(),
                positions: raw.positions,
                url: raw.url,
                is_animated: raw.is_animated,
                width: raw.width,
                height: raw.height,
                metadata: EmoteMetadata {
                    is_zero_width: raw.is_zero_width,
                    modifier: raw.modifier,
                    emote_set_id: raw.emote_set_id,
                    tier: raw.tier,
                },
            })
            .collect()
    }

    /// Emote en formato crudo desde cualquier plataforma
    #[derive(Debug, Clone)]
    pub struct RawEmote {
        pub id: String,
        pub name: String,
        pub positions: Vec<TextPosition>,
        pub url: Option<String>,
        pub is_animated: bool,
        pub width: Option<u32>,
        pub height: Option<u32>,
        pub is_zero_width: bool,
        pub modifier: bool,
        pub emote_set_id: Option<String>,
        pub tier: Option<String>,
    }

    /// Detecta si un texto contiene emotes conocidos
    pub fn detect_emotes_in_text(
        text: &str,
        known_emotes: &[String],
    ) -> Vec<(String, Vec<TextPosition>)> {
        let mut found_emotes = Vec::new();

        for emote_name in known_emotes {
            let mut positions = Vec::new();
            let mut start = 0;

            while let Some(pos) = text[start..].find(emote_name) {
                let actual_start = start + pos;
                let actual_end = actual_start + emote_name.len();

                // Verificar que esté como palabra completa
                let prev_char = if actual_start > 0 {
                    text.chars().nth(actual_start - 1)
                } else {
                    None
                };

                let next_char = text.chars().nth(actual_end);

                let is_word_boundary = match (prev_char, next_char) {
                    (None, None) => true,
                    (None, Some(c)) => !c.is_alphanumeric(),
                    (Some(c), None) => !c.is_alphanumeric(),
                    (Some(prev), Some(next)) => !prev.is_alphanumeric() && !next.is_alphanumeric(),
                };

                if is_word_boundary {
                    positions.push(TextPosition {
                        start: actual_start,
                        end: actual_end - 1,
                    });
                }

                start = actual_end;
            }

            if !positions.is_empty() {
                found_emotes.push((emote_name.clone(), positions));
            }
        }

        found_emotes
    }
}
