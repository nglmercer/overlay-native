use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::config::{Credentials, PlatformConfig, PlatformType};
use crate::connection::{
    Badge, ChatMessage, Emote, EmoteMetadata, EmoteSource, MessageMetadata, MessageType,
    StreamingPlatform, TextPosition,
};
use crate::platforms::base::{BasePlatform, ChannelInfo};
use crate::platforms::{utils, PlatformCreator, PlatformError, PlatformWrapperError};

#[derive(Debug, thiserror::Error)]
pub enum KickError {
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("API error: {0}")]
    ApiError(String),
}

pub struct KickPlatform {
    base: BasePlatform,
    websocket: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    channel_id: Option<String>,
    chatroom_id: Option<String>,
}

/// Kick WebSocket message structures
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum KickMessage {
    #[serde(rename = "message")]
    ChatMessage { data: ChatMessageData },
    #[serde(rename = "onroom")]
    OnRoom { data: RoomData },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessageData {
    id: String,
    content: String,
    sender: UserData,
    created_at: String,
    #[serde(default)]
    metadata: Option<MessageMetadataData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserData {
    id: String,
    username: String,
    displayname: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    badges: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RoomData {
    id: String,
    chatroom: ChatroomData,
    channel: ChannelData,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatroomData {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChannelData {
    id: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MessageMetadataData {
    #[serde(default)]
    badges: Vec<BadgeData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BadgeData {
    name: String,
    version: String,
}

impl KickPlatform {
    pub fn new(config: PlatformConfig) -> Result<Self, KickError> {
        let base = BasePlatform::new("kick".to_string(), PlatformType::Kick, config.clone());

        Ok(Self {
            base,
            websocket: None,
            channel_id: None,
            chatroom_id: None,
        })
    }

    async fn connect_to_websocket(&mut self) -> Result<(), KickError> {
        let url = Url::parse("wss://ws-us2.kick.com/chat/")
            .map_err(|e| KickError::WebSocketError(format!("Invalid WebSocket URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| KickError::WebSocketError(format!("Failed to connect: {}", e)))?;

        self.websocket = Some(ws_stream);
        Ok(())
    }

    async fn get_channel_info(&self, channel_name: &str) -> Result<(String, String), KickError> {
        let client = reqwest::Client::new();
        let url = format!("https://kick.com/api/v2/channels/{}", channel_name);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| KickError::ApiError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(KickError::ApiError(format!(
                "API request failed with status: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct ChannelResponse {
            id: String,
            chatroom: ChatroomResponse,
        }

        #[derive(Deserialize)]
        struct ChatroomResponse {
            id: String,
        }

        let channel_data: ChannelResponse = response
            .json()
            .await
            .map_err(|e| KickError::ParseError(format!("Failed to parse response: {}", e)))?;

        Ok((channel_data.id, channel_data.chatroom.id))
    }

    async fn join_chatroom(&mut self, channel_name: &str) -> Result<(), KickError> {
        let (channel_id, chatroom_id) = self.get_channel_info(channel_name).await?;
        self.channel_id = Some(channel_id.clone());
        self.chatroom_id = Some(chatroom_id.clone());

        if let Some(ws) = &mut self.websocket {
            let join_message = serde_json::json!({
                "event": "onroom",
                "data": {
                    "id": chatroom_id,
                    "channel": channel_id
                }
            });

            let message = Message::Text(join_message.to_string());
            ws.send(message).await.map_err(|e| {
                KickError::WebSocketError(format!("Failed to send join message: {}", e))
            })?;
        }

        Ok(())
    }

    fn convert_kick_message(&self, msg_data: ChatMessageData) -> ChatMessage {
        let badges = if let Some(metadata) = msg_data.metadata {
            metadata
                .badges
                .into_iter()
                .map(|b| Badge {
                    id: b.name.clone(),
                    name: b.name,
                    version: b.version,
                    url: None,
                    title: None,
                    source: EmoteSource::Kick,
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        ChatMessage {
            id: msg_data.id,
            platform: "kick".to_string(),
            channel: self
                .base
                .channels
                .keys()
                .next()
                .cloned()
                .unwrap_or_default(),
            username: msg_data.sender.username.clone(),
            display_name: Some(msg_data.sender.displayname),
            content: msg_data.content.clone(),
            emotes: Vec::new(), // Kick emotes would need additional parsing
            badges,
            timestamp: SystemTime::now(),
            user_color: msg_data.sender.color,
            message_type: MessageType::Normal,
            metadata: MessageMetadata {
                is_action: msg_data.content.starts_with("/me "),
                is_whisper: false,
                is_highlighted: false,
                is_me_message: msg_data.content.starts_with("/me"),
                reply_to: None,
                thread_id: None,
                custom_data: HashMap::new(),
            },
        }
    }
}

#[async_trait]
impl StreamingPlatform for KickPlatform {
    type Error = KickError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        self.connect_to_websocket().await?;
        self.base.connected = true;
        Ok(())
    }

    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        if !self.base.connected {
            self.connect().await?;
        }

        self.join_chatroom(&channel).await?;
        self.base.channels.insert(
            channel.clone(),
            ChannelInfo {
                name: channel.clone(),
                joined_at: Instant::now(),
                message_count: 0,
                viewer_count: None,
                live: true,
                title: None,
                category: None,
            },
        );

        Ok(())
    }

    async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        self.base.channels.remove(&channel);
        Ok(())
    }

    async fn next_message(&mut self) -> Option<ChatMessage> {
        if let Some(ws) = &mut self.websocket {
            while let Some(msg) = ws.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(kick_msg) = serde_json::from_str::<KickMessage>(&text) {
                            match kick_msg {
                                KickMessage::ChatMessage { data } => {
                                    return Some(self.convert_kick_message(data));
                                }
                                KickMessage::Ping => {
                                    // Respond with pong
                                    if let Err(_) = ws
                                        .send(Message::Text("{\"event\":\"pong\"}".to_string()))
                                        .await
                                    {
                                        return None;
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        self.base.connected = false;
                        return None;
                    }
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        return None;
                    }
                    _ => continue,
                }
            }
        }
        None
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        if let Some(ws) = &mut self.websocket {
            ws.close(None).await.map_err(|e| {
                KickError::WebSocketError(format!("Failed to close WebSocket: {}", e))
            })?;
        }
        self.websocket = None;
        self.base.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.connected
    }

    fn platform_name(&self) -> &str {
        "kick"
    }

    async fn get_channel_emotes(&self, _channel: &str) -> Result<Vec<Emote>, Self::Error> {
        // Kick emotes would need API integration
        Ok(Vec::new())
    }

    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
        // Kick global emotes would need API integration
        Ok(Vec::new())
    }

    fn parse_emotes(&self, _content: &str, _emote_data: &str) -> Vec<Emote> {
        // Kick emote parsing implementation
        Vec::new()
    }

    fn parse_badges(&self, _badge_data: &str) -> Vec<Badge> {
        // Kick badge parsing implementation
        Vec::new()
    }

    fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        self.base.apply_message_filters(message, filters)
    }
}

pub struct KickCreator;

#[async_trait]
impl PlatformCreator for KickCreator {
    async fn create(
        &self,
        config: crate::config::PlatformConfig,
    ) -> Result<
        Box<dyn crate::connection::StreamingPlatform<Error = PlatformWrapperError> + Send + Sync>,
        PlatformError,
    > {
        let platform =
            KickPlatform::new(config).map_err(|e| PlatformError::ConfigError(e.to_string()))?;

        // Wrap the platform to convert KickError to the expected error type
        let wrapped = KickPlatformWrapper::new(platform);
        Ok(Box::new(wrapped))
    }

    fn platform_name(&self) -> &str {
        "kick"
    }

    fn required_credentials(&self) -> Vec<&'static str> {
        vec![] // Kick doesn't require authentication for read-only chat
    }

    async fn validate_credentials(
        &self,
        _credentials: &crate::config::Credentials,
    ) -> Result<bool, PlatformError> {
        // Kick doesn't require credentials for basic chat access
        Ok(true)
    }
}

// Wrapper to convert KickError to the expected error type
pub struct KickPlatformWrapper {
    inner: KickPlatform,
}

impl KickPlatformWrapper {
    pub fn new(platform: KickPlatform) -> Self {
        Self { inner: platform }
    }
}

#[async_trait]
impl StreamingPlatform for KickPlatformWrapper {
    type Error = PlatformWrapperError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        self.inner
            .connect()
            .await
            .map_err(PlatformWrapperError::Kick)
    }

    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        self.inner
            .join_channel(channel)
            .await
            .map_err(PlatformWrapperError::Kick)
    }

    async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        self.inner
            .leave_channel(channel)
            .await
            .map_err(PlatformWrapperError::Kick)
    }

    async fn next_message(&mut self) -> Option<ChatMessage> {
        self.inner.next_message().await
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.inner
            .disconnect()
            .await
            .map_err(PlatformWrapperError::Kick)
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
            .map_err(PlatformWrapperError::Kick)
    }

    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
        self.inner
            .get_global_emotes()
            .await
            .map_err(PlatformWrapperError::Kick)
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
}

impl Default for KickPlatform {
    fn default() -> Self {
        Self::new(PlatformConfig::default()).unwrap()
    }
}
