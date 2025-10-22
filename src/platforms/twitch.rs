use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use tokio::sync::mpsc;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::{PrivmsgMessage, ServerMessage, TwitchUserBasics};
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};

use crate::config::{Credentials, PlatformConfig, PlatformType};
use crate::connection::{
    Badge, ChatMessage, Emote, EmoteMetadata, EmoteSource, MessageMetadata, MessageType,
    StreamingPlatform, TextPosition,
};
use crate::platforms::base::{emote_utils::RawEmote, BasePlatform, ChannelInfo};
use crate::platforms::{utils, PlatformCreator, PlatformError, PlatformWrapperError};

#[derive(Debug)]
pub enum TwitchError {
    ConnectionError(String),
    JoinError(String),
    AuthError(String),
    ParseError(String),
}

impl std::fmt::Display for TwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwitchError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            TwitchError::JoinError(msg) => write!(f, "Join error: {}", msg),
            TwitchError::AuthError(msg) => write!(f, "Auth error: {}", msg),
            TwitchError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for TwitchError {}

pub struct TwitchPlatform {
    base: BasePlatform,
    client: Option<TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>>,
    message_receiver: Option<mpsc::UnboundedReceiver<ServerMessage>>,
}

impl TwitchPlatform {
    pub fn new(config: PlatformConfig) -> Result<Self, TwitchError> {
        let mut base =
            BasePlatform::new("twitch".to_string(), PlatformType::Twitch, config.clone());

        // Credenciales son opcionales para conexiones anónimas

        Ok(Self {
            base,
            client: None,
            message_receiver: None,
        })
    }

    fn convert_twitch_emotes(emotes: &[twitch_irc::message::Emote]) -> Vec<Emote> {
        emotes
            .iter()
            .map(|emote| {
                let source = if emote.id.starts_with("emotesv2_") {
                    EmoteSource::TwitchGlobal
                } else if emote.id.chars().all(|c| c.is_ascii_digit()) {
                    EmoteSource::TwitchSubscriber
                } else {
                    EmoteSource::Twitch
                };

                Emote {
                    id: emote.id.clone(),
                    name: emote.code.clone(),
                    source,
                    positions: vec![TextPosition {
                        start: emote.char_range.start,
                        end: emote.char_range.end,
                    }],
                    url: Some(format!(
                        "https://static-cdn.jtvnw.net/emoticons/v2/{}",
                        emote.id
                    )),
                    is_animated: false, // Twitch no indica esto en el mensaje base
                    width: Some(28),
                    height: Some(28),
                    metadata: EmoteMetadata {
                        is_zero_width: false,
                        modifier: false,
                        emote_set_id: Some(emote.id.clone()),
                        tier: None, // Se podría obtener de la API
                    },
                }
            })
            .collect()
    }

    fn convert_twitch_badges(badges: &[twitch_irc::message::Badge]) -> Vec<Badge> {
        badges
            .iter()
            .map(|badge| Badge {
                id: badge.name.clone(),
                name: badge.name.clone(),
                version: badge.version.clone(),
                url: Some(format!(
                    "https://static-cdn.jtvnw.net/badges/v1/{}/{}",
                    badge.name, badge.version
                )),
                title: None,
                source: EmoteSource::Twitch,
            })
            .collect()
    }

    fn convert_privmsg_message(msg: PrivmsgMessage) -> ChatMessage {
        let message_type = if msg.message_text.starts_with("/me") {
            MessageType::Action
        } else if msg.message_text.starts_with('!') {
            MessageType::Normal // Podría ser comando, pero lo tratamos como normal
        } else {
            MessageType::Normal
        };

        let mut metadata = MessageMetadata {
            is_action: msg.message_text.starts_with("/me"),
            is_whisper: false,
            is_highlighted: false,
            is_me_message: msg.message_text.starts_with("/me"),
            reply_to: None, // TODO: Fix reply field access when available
            thread_id: None,
            custom_data: HashMap::new(),
        };

        // Agregar datos específicos de Twitch
        metadata
            .custom_data
            .insert("user_id".to_string(), msg.sender.id.into());
        metadata
            .custom_data
            .insert("message_id".to_string(), msg.message_id.clone().into());

        metadata
            .custom_data
            .insert("room_id".to_string(), msg.channel_id.clone().into());

        ChatMessage {
            id: msg.message_id.to_string(),
            platform: "twitch".to_string(),
            channel: msg.channel_login,
            username: msg.sender.login.clone(),
            display_name: Some(msg.sender.name.clone()),
            content: msg.message_text.clone(),
            emotes: Self::convert_twitch_emotes(&msg.emotes),
            badges: Self::convert_twitch_badges(&msg.badges),
            timestamp: SystemTime::now(),
            user_color: None,
            message_type,
            metadata,
        }
    }

    async fn handle_server_message(&mut self, message: ServerMessage) -> Option<ChatMessage> {
        eprintln!("[DEBUG] Received Twitch message: {:?}", message);
        match message {
            ServerMessage::Privmsg(privmsg) => {
                eprintln!(
                    "[DEBUG] Processing PRIVMSG from {}: {}",
                    privmsg.sender.name, privmsg.message_text
                );
                // Actualizar estadísticas del canal
                let channel_login = privmsg.channel_login.clone();
                if let Some(channel_info) = self.base.get_channel_info(&channel_login) {
                    let mut updated_info = channel_info.clone();
                    updated_info.message_count += 1;
                    self.base.update_channel_info(channel_login, updated_info);
                }

                Some(Self::convert_privmsg_message(privmsg))
            }
            ServerMessage::ClearChat(msg) => {
                // Mensaje de sistema de timeout/ban
                Some(ChatMessage {
                    id: utils::generate_message_id(),
                    platform: "twitch".to_string(),
                    channel: msg.channel_login,
                    username: "system".to_string(),
                    display_name: Some("System".to_string()),
                    content: match &msg.action {
                        twitch_irc::message::ClearChatAction::UserBanned { user_login, .. } => {
                            format!("{} has been banned", user_login)
                        }
                        twitch_irc::message::ClearChatAction::UserTimedOut {
                            user_login, ..
                        } => {
                            format!("{} has been timed out", user_login)
                        }
                        twitch_irc::message::ClearChatAction::ChatCleared => {
                            "Chat has been cleared by a moderator".to_string()
                        }
                    },
                    emotes: Vec::new(),
                    badges: Vec::new(),
                    timestamp: SystemTime::now(),
                    user_color: Some("#ff0000".to_string()),
                    message_type: MessageType::System,
                    metadata: MessageMetadata {
                        is_action: false,
                        is_whisper: false,
                        is_highlighted: true,
                        is_me_message: false,
                        reply_to: None,
                        thread_id: None,
                        custom_data: {
                            let mut data = HashMap::new();
                            data.insert("clear_type".to_string(), "chat".into());
                            data
                        },
                    },
                })
            }
            ServerMessage::UserNotice(msg) => {
                // Mensajes de suscripción, raid, etc.
                let message_content = match msg.message_id.as_str() {
                    "sub" | "resub" => {
                        // For subscription messages, we'll use a generic message
                        // since cumulative_months may not be available in all cases
                        format!("{} has subscribed!", msg.sender.name)
                    }
                    "raid" => {
                        format!("{} is raiding the channel!", msg.sender.name)
                    }
                    _ => format!("System notice from {}", msg.sender.name),
                };

                Some(ChatMessage {
                    id: utils::generate_message_id(),
                    platform: "twitch".to_string(),
                    channel: msg.channel_login,
                    username: "system".to_string(),
                    display_name: Some("System".to_string()),
                    content: message_content,
                    emotes: Vec::new(),
                    badges: Vec::new(),
                    timestamp: SystemTime::now(),
                    user_color: Some("#00ff00".to_string()),
                    message_type: MessageType::Subscription,
                    metadata: MessageMetadata {
                        is_action: false,
                        is_whisper: false,
                        is_highlighted: true,
                        is_me_message: false,
                        reply_to: None,
                        thread_id: None,
                        custom_data: {
                            let mut data = HashMap::new();
                            data.insert("notice_type".to_string(), msg.message_id.into());
                            data
                        },
                    },
                })
            }
            ServerMessage::RoomState(msg) => {
                // Actualizar información del canal
                let channel_info = ChannelInfo {
                    name: msg.channel_login.clone(),
                    joined_at: Instant::now(),
                    message_count: 0,
                    viewer_count: Some(0), // No viene en RoomState
                    live: true,
                    title: None,
                    category: None,
                };
                self.base
                    .update_channel_info(msg.channel_login, channel_info);
                None
            }
            ServerMessage::Ping(_) | ServerMessage::Pong(_) => {
                // Ignorar mensajes de ping/pong
                None
            }
            _ => {
                // Otros mensajes pueden ser loggeados si es necesario
                None
            }
        }
    }
}

#[async_trait]
impl StreamingPlatform for TwitchPlatform {
    type Error = TwitchError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        // Use anonymous connection if no credentials are provided
        let username = self
            .base
            .credentials
            .username
            .clone()
            .unwrap_or_else(|| "justinfan12345".to_string());

        let oauth_token = self.base.credentials.oauth_token.clone();

        let credentials = if let Some(token) = oauth_token {
            if token.is_empty() || token == "oauth:YOUR_OAUTH_TOKEN_HERE" {
                // Use anonymous connection with default username
                StaticLoginCredentials::new("justinfan12345".to_string(), None)
            } else {
                // Use provided credentials
                StaticLoginCredentials::new(username, Some(token))
            }
        } else {
            // No oauth token provided, use anonymous connection
            StaticLoginCredentials::new("justinfan12345".to_string(), None)
        };

        let config = ClientConfig::new_simple(credentials);
        let (incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

        self.client = Some(client);
        self.message_receiver = Some(incoming_messages);
        self.base.connected = true;

        Ok(())
    }

    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        if let Some(client) = &self.client {
            let sanitized_channel = utils::sanitize_channel_name(&channel);

            client
                .join(sanitized_channel.clone())
                .map_err(|e| TwitchError::JoinError(e.to_string()))?;

            // Agregar canal a la lista de canales activos
            let channel_info = ChannelInfo {
                name: sanitized_channel.clone(),
                joined_at: Instant::now(),
                message_count: 0,
                viewer_count: None,
                live: false,
                title: None,
                category: None,
            };
            self.base
                .update_channel_info(sanitized_channel.clone(), channel_info);

            Ok(())
        } else {
            Err(TwitchError::ConnectionError(
                "Not connected to Twitch".to_string(),
            ))
        }
    }

    async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        if let Some(client) = &self.client {
            let sanitized_channel = utils::sanitize_channel_name(&channel);

            client.part(sanitized_channel.clone());

            // Remover canal de la lista
            self.base.channels.remove(&sanitized_channel);

            Ok(())
        } else {
            Err(TwitchError::ConnectionError(
                "Not connected to Twitch".to_string(),
            ))
        }
    }

    async fn next_message(&mut self) -> Option<ChatMessage> {
        loop {
            let message = match &mut self.message_receiver {
                Some(receiver) => receiver.recv().await,
                None => {
                    eprintln!("[DEBUG] No message receiver available");
                    return None;
                }
            };

            if let Some(message) = message {
                eprintln!("[DEBUG] Raw message received from Twitch IRC");
                if let Some(chat_message) = self.handle_server_message(message).await {
                    eprintln!(
                        "[DEBUG] Converted to ChatMessage: {} - {}",
                        chat_message.username, chat_message.content
                    );
                    return Some(chat_message);
                } else {
                    eprintln!("[DEBUG] Message filtered out or not converted");
                }
            } else {
                eprintln!("[DEBUG] No message received (channel closed)");
                return None;
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.base.connected = false;
        self.client = None;
        self.message_receiver = None;
        self.base.channels.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.connected
    }

    fn platform_name(&self) -> &str {
        &self.base.platform_name
    }

    async fn get_channel_emotes(&self, _channel: &str) -> Result<Vec<Emote>, Self::Error> {
        // Esto requeriría llamadas a la API de Twitch
        // Por ahora, devolvemos una lista vacía
        Ok(Vec::new())
    }

    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
        // Esto requeriría llamadas a la API de Twitch
        // Por ahora, devolvemos una lista vacía
        Ok(Vec::new())
    }

    fn parse_emotes(&self, content: &str, emote_data: &str) -> Vec<Emote> {
        self.base.parse_generic_emotes(content, emote_data)
    }

    fn parse_badges(&self, badge_data: &str) -> Vec<Badge> {
        self.base.parse_generic_badges(badge_data)
    }

    fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        self.base.apply_message_filters(message, filters)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Creador de plataforma Twitch
pub struct TwitchCreator;

// Wrapper to convert TwitchError to the expected error type
pub struct TwitchPlatformWrapper {
    inner: TwitchPlatform,
}

impl TwitchPlatformWrapper {
    pub fn new(platform: TwitchPlatform) -> Self {
        Self { inner: platform }
    }
}

#[async_trait]
impl StreamingPlatform for TwitchPlatformWrapper {
    type Error = PlatformWrapperError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        self.inner
            .connect()
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        self.inner
            .join_channel(channel)
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        self.inner
            .leave_channel(channel)
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    async fn next_message(&mut self) -> Option<ChatMessage> {
        self.inner.next_message().await
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.inner
            .disconnect()
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    fn platform_name(&self) -> &str {
        self.inner.platform_name()
    }

    async fn get_channel_emotes(&self, channel: &str) -> Result<Vec<Emote>, Self::Error> {
        self.inner
            .get_channel_emotes(channel)
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
        self.inner
            .get_global_emotes()
            .await
            .map_err(PlatformWrapperError::Twitch)
    }

    fn parse_emotes(&self, content: &str, emote_data: &str) -> Vec<Emote> {
        self.inner.parse_emotes(content, emote_data)
    }

    fn parse_badges(&self, badge_data: &str) -> Vec<Badge> {
        self.inner.parse_badges(badge_data)
    }

    fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        self.inner.apply_message_filters(message, filters)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        &mut self.inner
    }
}

#[async_trait]
impl PlatformCreator for TwitchCreator {
    async fn create(
        &self,
        config: PlatformConfig,
    ) -> Result<Box<dyn StreamingPlatform<Error = PlatformWrapperError> + Send + Sync>, PlatformError>
    {
        let platform =
            TwitchPlatform::new(config).map_err(|e| PlatformError::ConfigError(e.to_string()))?;

        // Wrap the platform to convert TwitchError to the expected error type
        let wrapped = TwitchPlatformWrapper::new(platform);
        Ok(Box::new(wrapped))
    }

    fn platform_name(&self) -> &str {
        "twitch"
    }

    fn required_credentials(&self) -> Vec<&'static str> {
        vec![] // No credentials required for anonymous connections
    }

    async fn validate_credentials(&self, credentials: &Credentials) -> Result<bool, PlatformError> {
        // Allow anonymous connections (no credentials) or validate provided credentials
        if credentials.username.is_none() && credentials.oauth_token.is_none() {
            // Anonymous connection is allowed
            Ok(true)
        } else if let (Some(username), Some(token)) =
            (&credentials.username, &credentials.oauth_token)
        {
            // Validate provided credentials
            Ok(!username.is_empty() && token.starts_with("oauth:") && token.len() > 10)
        } else {
            // Invalid combination (only one credential provided)
            Ok(false)
        }
    }
}

impl Default for TwitchPlatform {
    fn default() -> Self {
        Self::new(PlatformConfig::default()).unwrap()
    }
}
