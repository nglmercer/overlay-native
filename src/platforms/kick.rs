use async_trait::async_trait;
use kick_rust::KickClient;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, Mutex};

use crate::config::{PlatformConfig, PlatformType};
use crate::connection::{
    Badge, ChatMessage, Emote, MessageMetadata, MessageType, StreamingPlatform,
};
use crate::platforms::base::BasePlatform;
use crate::platforms::{PlatformCreator, PlatformError, PlatformWrapperError};

#[derive(Debug, thiserror::Error)]
pub enum KickError {
    #[error("Kick client error: {0}")]
    ClientError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Channel error: {0}")]
    ChannelError(String),
}

pub struct KickPlatform {
    base: BasePlatform,
    client: Option<KickClient>,
    current_channel: Option<String>,
    message_receiver: Option<mpsc::UnboundedReceiver<ChatMessage>>,
    message_sender: Option<mpsc::UnboundedSender<ChatMessage>>,
    is_connected: bool,
    config: PlatformConfig,
}

impl KickPlatform {
    pub fn new(config: PlatformConfig) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Self {
            base: BasePlatform::new(
                "Kick".to_string(),
                PlatformType::Kick,
                config.clone(),
            ),
            client: None,
            current_channel: None,
            message_receiver: Some(message_receiver),
            message_sender: Some(message_sender),
            is_connected: false,
            config,
        }
    }

    pub fn set_auth_tokens(&mut self, _bearer_token: String, _xsrf_token: String, _cookies: String) {
        // Note: kick_rust library handles authentication internally
        // This method is kept for compatibility but may not be needed
    }

    pub fn clear_auth_tokens(&mut self) {
        // Note: kick_rust library handles authentication internally
        // This method is kept for compatibility but may not be needed
    }

    pub fn set_channel_ids(&mut self, _channel_id: String, _chatroom_id: String) {
        // Note: kick_rust library handles channel resolution internally
        // This method is kept for compatibility but may not be needed
    }

    async fn setup_callbacks(&mut self) -> Result<(), KickError> {
        if let Some(client) = &self.client {
            if let Some(sender) = self.message_sender.take() {
                let sender = Arc::new(Mutex::new(sender));

                // Handle chat messages
                let sender_clone = Arc::clone(&sender);
                client.on_chat_message(move |data| {
                    let chat_message = ChatMessage {
                        id: data.id.clone(),
                        platform: "Kick".to_string(),
                        channel: "unknown".to_string(), // Will be set when joining channel
                        username: data.sender.username.clone(),
                        display_name: Some(data.sender.username.clone()),
                        content: data.content.clone(),
                        emotes: Vec::new(), // TODO: Parse emotes from kick_rust if available
                        badges: Vec::new(), // TODO: Parse badges from kick_rust if available
                        timestamp: SystemTime::now(),
                        user_color: None, // TODO: Get user color from kick_rust if available
                        message_type: MessageType::Normal,
                        metadata: MessageMetadata {
                            is_action: false,
                            is_whisper: false,
                            is_highlighted: false,
                            is_me_message: false,
                            reply_to: None,
                            thread_id: None,
                            custom_data: HashMap::new(),
                        },
                    };

                    if let Ok(sender) = sender_clone.try_lock() {
                        let _ = sender.send(chat_message);
                    }
                }).await;

                // Handle connection ready
                client.on_ready(move |_| {
                    println!("Connected to Kick chat!");
                }).await;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StreamingPlatform for KickPlatform {
    type Error = KickError;

    async fn connect(&mut self) -> Result<(), Self::Error> {
        let client = KickClient::new();
        self.client = Some(client);

        self.setup_callbacks().await?;
        self.is_connected = true;

        Ok(())
    }

    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        if let Some(client) = &self.client {
            client.connect(&channel).await
                .map_err(|e| KickError::ConnectionError(e.to_string()))?;

            self.current_channel = Some(channel);
            Ok(())
        } else {
            Err(KickError::ClientError("Client not initialized".to_string()))
        }
    }

    async fn leave_channel(&mut self, _channel: String) -> Result<(), Self::Error> {
        // Note: kick_rust library may not have explicit leave_channel method
        // This is a placeholder implementation
        self.current_channel = None;
        Ok(())
    }

    async fn next_message(&mut self) -> Option<ChatMessage> {
        if let Some(receiver) = &mut self.message_receiver {
            receiver.recv().await
        } else {
            None
        }
    }

    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        // Note: kick_rust library may not have explicit disconnect method
        // This is a placeholder implementation
        self.is_connected = false;
        self.current_channel = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected
    }

    fn platform_name(&self) -> &str {
        "Kick"
    }

    async fn get_channel_emotes(&self, _channel: &str) -> Result<Vec<Emote>, Self::Error> {
        // Note: kick_rust library may not have emote support yet
        // Return empty vector for now
        Ok(Vec::new())
    }

    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
        // Note: kick_rust library may not have emote support yet
        // Return empty vector for now
        Ok(Vec::new())
    }

    fn parse_emotes(&self, _content: &str, _emote_data: &str) -> Vec<Emote> {
        // Note: kick_rust library may not have emote support yet
        // Return empty vector for now
        Vec::new()
    }

    fn parse_badges(&self, _badge_data: &str) -> Vec<Badge> {
        // Note: kick_rust library may not have badge support yet
        // Return empty vector for now
        Vec::new()
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

pub struct KickCreator;

#[async_trait]
impl PlatformCreator for KickCreator {
    async fn create(
        &self,
        config: PlatformConfig,
    ) -> Result<Box<dyn StreamingPlatform<Error = PlatformWrapperError> + Send + Sync>, PlatformError>
    {
        let platform = KickPlatform::new(config);

        // Wrap the platform to convert KickError to the expected error type
        let wrapped = KickPlatformWrapper::new(platform);
        Ok(Box::new(wrapped))
    }

    fn platform_name(&self) -> &str {
        "Kick"
    }

    fn required_credentials(&self) -> Vec<&'static str> {
        // Note: kick_rust library may not require explicit credentials
        Vec::new()
    }

    async fn validate_credentials(&self, _credentials: &crate::config::Credentials) -> Result<bool, PlatformError> {
        // Note: kick_rust library handles authentication internally
        Ok(true)
    }
}

pub struct KickPlatformWrapper {
    inner: KickPlatform,
}

impl KickPlatformWrapper {
    pub fn new(platform: KickPlatform) -> Self {
        Self {
            inner: platform,
        }
    }

    fn parse_emotes(&self, content: &str, emote_data: &str) -> Vec<Emote> {
        self.inner.parse_emotes(content, emote_data)
    }

    fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool {
        self.inner.apply_message_filters(message, filters)
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for KickPlatform {
    fn default() -> Self {
        Self::new(PlatformConfig::default())
    }
}
