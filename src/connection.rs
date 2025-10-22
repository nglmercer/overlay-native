use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Type alias for platform errors to simplify trait bounds
pub type PlatformError = Box<dyn std::error::Error + Send + Sync>;

fn system_time_now() -> SystemTime {
    SystemTime::now()
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub platform: String,
    pub channel: String,
    pub username: String,
    pub display_name: Option<String>,
    pub content: String,
    pub emotes: Vec<Emote>,
    pub badges: Vec<Badge>,
    #[serde(default = "system_time_now")]
    #[serde_as(as = "serde_with::TimestampSeconds<i64>")]
    pub timestamp: SystemTime,
    pub user_color: Option<String>,
    pub message_type: MessageType,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emote {
    pub id: String,
    pub name: String,
    pub source: EmoteSource,
    pub positions: Vec<TextPosition>,
    pub url: Option<String>,
    pub is_animated: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub metadata: EmoteMetadata,
}

impl Default for Emote {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            source: EmoteSource::Local,
            positions: Vec::new(),
            url: None,
            is_animated: false,
            width: None,
            height: None,
            metadata: EmoteMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub source: EmoteSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmoteMetadata {
    pub is_zero_width: bool,
    pub modifier: bool,
    pub emote_set_id: Option<String>,
    pub tier: Option<String>, // para subscriber emotes
}

impl Default for EmoteMetadata {
    fn default() -> Self {
        Self {
            is_zero_width: false,
            modifier: false,
            emote_set_id: None,
            tier: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub is_action: bool,
    pub is_whisper: bool,
    pub is_highlighted: bool,
    pub is_me_message: bool,
    pub reply_to: Option<String>,
    pub thread_id: Option<String>,
    pub custom_data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
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
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(PartialEq, Eq, Hash)]
pub enum EmoteSource {
    Twitch,
    TwitchGlobal,
    TwitchSubscriber,
    BTTV,
    FFZ,
    SevenTV,
    YouTube,
    YouTubeCustom,
    Kick,
    Trovo,
    Facebook,
    Local,
}

impl std::fmt::Display for EmoteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmoteSource::Twitch => write!(f, "twitch"),
            EmoteSource::TwitchGlobal => write!(f, "twitch_global"),
            EmoteSource::TwitchSubscriber => write!(f, "twitch_subscriber"),
            EmoteSource::BTTV => write!(f, "bttv"),
            EmoteSource::FFZ => write!(f, "ffz"),
            EmoteSource::SevenTV => write!(f, "7tv"),
            EmoteSource::YouTube => write!(f, "youtube"),
            EmoteSource::YouTubeCustom => write!(f, "youtube_custom"),
            EmoteSource::Kick => write!(f, "kick"),
            EmoteSource::Trovo => write!(f, "trovo"),
            EmoteSource::Facebook => write!(f, "facebook"),
            EmoteSource::Local => write!(f, "local"),
        }
    }
}

#[async_trait]
pub trait StreamingPlatform {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Conecta a la plataforma de streaming
    async fn connect(&mut self) -> Result<(), Self::Error>;

    /// Se une a un canal específico
    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error>;

    /// Abandona un canal específico
    async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error>;

    /// Obtiene el siguiente mensaje del chat
    async fn next_message(&mut self) -> Option<ChatMessage>;

    /// Desconecta de la plataforma
    async fn disconnect(&mut self) -> Result<(), Self::Error>;

    /// Verifica si la conexión está activa
    fn is_connected(&self) -> bool;

    /// Obtiene el nombre de la plataforma
    fn platform_name(&self) -> &str;

    /// Obtiene emotes disponibles para un canal
    async fn get_channel_emotes(&self, channel: &str) -> Result<Vec<Emote>, Self::Error>;

    /// Obtiene emotes globales de la plataforma
    async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error>;

    /// Parsea emotes en un mensaje
    fn parse_emotes(&self, content: &str, emote_data: &str) -> Vec<Emote>;

    /// Parsea badges de un usuario
    fn parse_badges(&self, badge_data: &str) -> Vec<Badge>;

    /// Aplica filtros de mensaje
    fn apply_message_filters(
        &self,
        message: &mut ChatMessage,
        filters: &crate::config::MessageFilters,
    ) -> bool;

    /// Permite downcasting a tipos concretos para acceder a métodos específicos
    fn as_any(&self) -> &dyn std::any::Any;

    /// Permite downcasting mutable a tipos concretos para acceder a métodos específicos
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct PlatformManager {
    message_sender: mpsc::UnboundedSender<ChatMessage>,
    message_receiver: mpsc::UnboundedReceiver<ChatMessage>,
    platforms: HashMap<
        String,
        std::sync::Arc<
            tokio::sync::Mutex<
                Box<
                    dyn StreamingPlatform<Error = crate::platforms::PlatformWrapperError>
                        + Send
                        + Sync,
                >,
            >,
        >,
    >,
    connections: HashMap<String, ConnectionInfo>,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub platform: String,
    pub channel: String,
    pub enabled: bool,
    pub display_name: Option<String>,
}

impl PlatformManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            message_sender: sender,
            message_receiver: receiver,
            platforms: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    pub fn get_sender(&self) -> mpsc::UnboundedSender<ChatMessage> {
        self.message_sender.clone()
    }

    pub async fn next_message(&mut self) -> Option<ChatMessage> {
        self.message_receiver.recv().await
    }

    pub fn register_platform(
        &mut self,
        name: String,
        platform: Box<
            dyn StreamingPlatform<Error = crate::platforms::PlatformWrapperError>
                + Send
                + Sync
                + 'static,
        >,
    ) {
        eprintln!("[DEBUG] Registering platform: {}", name);
        self.platforms
            .insert(name, std::sync::Arc::new(tokio::sync::Mutex::new(platform)));
        eprintln!(
            "[DEBUG] Total registered platforms: {}",
            self.platforms.len()
        );
    }

    pub fn add_connection(&mut self, info: ConnectionInfo) {
        eprintln!("[DEBUG] Adding connection: {:?}", info);
        self.connections.insert(info.id.clone(), info);
        eprintln!(
            "[DEBUG] Total connections after add: {}",
            self.connections.len()
        );
    }

    pub fn get_platform_mut(
        &mut self,
        platform_name: &str,
    ) -> Option<
        &mut std::sync::Arc<
            tokio::sync::Mutex<
                Box<
                    dyn StreamingPlatform<Error = crate::platforms::PlatformWrapperError>
                        + Send
                        + Sync,
                >,
            >,
        >,
    > {
        self.platforms.get_mut(platform_name)
    }

    pub async fn start_connection(
        &mut self,
        connection_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        eprintln!("[DEBUG] Attempting to start connection: {}", connection_id);
        eprintln!(
            "[DEBUG] Available connections: {:?}",
            self.connections.keys().collect::<Vec<_>>()
        );
        let connection_info = self
            .connections
            .get(connection_id)
            .ok_or("Connection not found")?
            .clone();
        eprintln!("[DEBUG] Connection info: {:?}", connection_info);

        if !connection_info.enabled {
            eprintln!("[DEBUG] Connection disabled: {}", connection_id);
            return Err("Connection is disabled".into());
        }

        eprintln!("[DEBUG] Looking for platform: {}", connection_info.platform);
        eprintln!(
            "[DEBUG] Available platforms: {:?}",
            self.platforms.keys().collect::<Vec<_>>()
        );
        let platform_arc = self
            .platforms
            .get(&connection_info.platform)
            .ok_or("Platform not found")?
            .clone();

        {
            let mut platform = platform_arc.lock().await;
            if !platform.is_connected() {
                eprintln!("[DEBUG] Platform not connected, connecting...");
                platform.connect().await?;
                eprintln!("[DEBUG] Platform connected successfully.");
            } else {
                eprintln!("[DEBUG] Platform already connected.");
            }
        }

        eprintln!(
            "[DEBUG] Joining channel: {}",
            connection_info.channel.clone()
        );
        {
            let mut platform = platform_arc.lock().await;
            platform
                .join_channel(connection_info.channel.clone())
                .await?;
        }
        eprintln!("[DEBUG] Joined channel: {}", connection_info.channel);

        let sender = self.message_sender.clone();
        let platform_name = connection_info.platform.clone();
        let channel = connection_info.channel.clone();

        tokio::spawn(async move {
            eprintln!(
                "[DEBUG] Spawned task for connection {} on channel {}. Starting message loop...",
                platform_name, channel
            );

            let mut message_count = 0;
            loop {
                // Get message without holding the lock for too long
                let message = {
                    let mut platform = platform_arc.lock().await;
                    platform.next_message().await
                };

                if let Some(mut message) = message {
                    message_count += 1;
                    eprintln!(
                        "[DEBUG] Received message #{} from {}: {} - {}",
                        message_count, platform_name, message.username, message.content
                    );
                    message.platform = platform_name.clone();
                    message.channel = channel.clone();

                    if sender.send(message).is_err() {
                        eprintln!("[DEBUG] Failed to send message, breaking loop");
                        break;
                    } else {
                        eprintln!("[DEBUG] Message sent successfully to manager");
                    }
                } else {
                    eprintln!("[DEBUG] No message received from platform");
                    // Small delay to prevent busy waiting
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
            eprintln!(
                "[DEBUG] Message loop ended for {} on channel {} (total messages: {})",
                platform_name, channel, message_count
            );
        });

        Ok(())
    }

    pub async fn run_platform<P: StreamingPlatform + Send + 'static>(
        &mut self,
        platform_name: String,
        mut platform: P,
        channel: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        eprintln!(
            "[DEBUG] run_platform called for {} on channel {}",
            platform_name, channel
        );
        platform.connect().await?;
        eprintln!("[DEBUG] Platform connected successfully");
        platform.join_channel(channel.clone()).await?;
        eprintln!("[DEBUG] Joined channel successfully");

        let sender = self.message_sender.clone();
        let platform_name_clone = platform_name.clone();

        tokio::spawn(async move {
            eprintln!(
                "[DEBUG] Starting message loop for {} on channel {}",
                platform_name_clone, channel
            );
            let mut message_count = 0;
            while platform.is_connected() {
                if let Some(mut message) = platform.next_message().await {
                    message_count += 1;
                    eprintln!(
                        "[DEBUG] Received message #{} from {}: {:?}",
                        message_count, platform_name_clone, message.content
                    );
                    message.platform = platform_name_clone.clone();
                    message.channel = channel.clone();

                    if sender.send(message).is_err() {
                        eprintln!("[DEBUG] Failed to send message, breaking loop");
                        break;
                    } else {
                        eprintln!("[DEBUG] Message sent successfully to manager");
                    }
                } else {
                    eprintln!("[DEBUG] No message received from platform");
                }
            }
            eprintln!(
                "[DEBUG] Message loop ended for {} on channel {} (total messages: {})",
                platform_name_clone, channel, message_count
            );
        });

        Ok(())
    }

    pub fn get_platform_names(&self) -> Vec<String> {
        self.platforms.keys().cloned().collect()
    }

    pub fn get_platform(
        &self,
        platform_name: &str,
    ) -> Option<
        &std::sync::Arc<
            tokio::sync::Mutex<
                Box<
                    dyn StreamingPlatform<Error = crate::platforms::PlatformWrapperError>
                        + Send
                        + Sync,
                >,
            >,
        >,
    > {
        self.platforms.get(platform_name)
    }

    pub fn get_connections(&self) -> Vec<&ConnectionInfo> {
        self.connections.values().collect()
    }

    pub fn get_enabled_connections(&self) -> Vec<&ConnectionInfo> {
        self.connections
            .values()
            .filter(|conn| conn.enabled)
            .collect()
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (_, platform) in &mut self.platforms {
            platform.lock().await.disconnect().await?;
        }
        Ok(())
    }
}

impl Default for PlatformManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilidades para procesamiento de mensajes
pub mod utils {
    use super::*;

    pub fn extract_emote_positions(emotes: &[Emote]) -> Vec<TextPosition> {
        emotes.iter().flat_map(|e| e.positions.clone()).collect()
    }

    pub fn replace_emotes_with_placeholders(content: &str, emotes: &[Emote]) -> String {
        let mut result = content.to_string();

        // Ordenar emotes por posición de inicio en orden descendente
        let mut sorted_emotes = emotes.to_vec();
        sorted_emotes.sort_by(|a, b| {
            let a_start = a.positions.first().map(|p| p.start).unwrap_or(0);
            let b_start = b.positions.first().map(|p| p.start).unwrap_or(0);
            b_start.cmp(&a_start)
        });

        for emote in sorted_emotes {
            for position in &emote.positions {
                if position.start < result.len() && position.end <= result.len() {
                    result.replace_range(position.start..position.end, &format!(":{}", emote.name));
                }
            }
        }

        result
    }

    pub fn calculate_message_metrics(message: &ChatMessage) -> MessageMetrics {
        MessageMetrics {
            word_count: message.content.split_whitespace().count(),
            emote_count: message.emotes.len(),
            badge_count: message.badges.len(),
            character_count: message.content.chars().count(),
            has_links: contains_links(&message.content),
            is_mentioned: is_mentioned(&message.content),
        }
    }

    fn contains_links(content: &str) -> bool {
        content.contains("http://") || content.contains("https://")
    }

    fn is_mentioned(content: &str) -> bool {
        content.to_lowercase().contains("@")
            && (content.to_lowercase().contains("@everyone")
                || content.to_lowercase().contains("@here"))
    }
}

#[derive(Debug, Clone)]
pub struct MessageMetrics {
    pub word_count: usize,
    pub emote_count: usize,
    pub badge_count: usize,
    pub character_count: usize,
    pub has_links: bool,
    pub is_mentioned: bool,
}

/// Sistema de cache de emotes
pub struct EmoteCache {
    cache: HashMap<String, Emote>,
    ttl: std::time::Duration,
    last_update: std::time::Instant,
}

impl EmoteCache {
    pub fn new(ttl_hours: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: std::time::Duration::from_secs(ttl_hours * 3600),
            last_update: std::time::Instant::now(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Emote> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: String, emote: Emote) {
        self.cache.insert(key, emote);
    }

    pub fn is_expired(&self) -> bool {
        self.last_update.elapsed() > self.ttl
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.last_update = std::time::Instant::now();
    }

    pub fn get_by_source(&self, source: &EmoteSource) -> Vec<&Emote> {
        self.cache
            .values()
            .filter(|emote| &emote.source == source)
            .collect()
    }
}

impl Default for EmoteCache {
    fn default() -> Self {
        Self::new(24) // 24 horas por defecto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MessageFilters;
    use crate::platforms::PlatformWrapperError;
    use async_trait::async_trait;
    use tokio::sync::mpsc as tokio_mpsc;

    // Mock StreamingPlatform
    #[derive(Debug)]
    struct MockPlatform {
        is_connected: bool,
        message_receiver: tokio_mpsc::UnboundedReceiver<ChatMessage>,
    }

    struct MockPlatformHandle {
        platform: MockPlatform,
        message_sender: tokio_mpsc::UnboundedSender<ChatMessage>,
    }

    impl MockPlatformHandle {
        fn new() -> Self {
            let (tx, rx) = tokio_mpsc::unbounded_channel();
            Self {
                platform: MockPlatform {
                    is_connected: false,
                    message_receiver: rx,
                },
                message_sender: tx,
            }
        }
    }

    #[async_trait]
    impl StreamingPlatform for MockPlatform {
        type Error = PlatformWrapperError;

        async fn connect(&mut self) -> Result<(), Self::Error> {
            eprintln!("[TEST] MockPlatform: connect");
            self.is_connected = true;
            Ok(())
        }

        async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
            eprintln!("[TEST] MockPlatform: join_channel {}", channel);
            Ok(())
        }

        async fn leave_channel(&mut self, channel: String) -> Result<(), Self::Error> {
            eprintln!("[TEST] MockPlatform: leave_channel {}", channel);
            Ok(())
        }

        async fn next_message(&mut self) -> Option<ChatMessage> {
            eprintln!("[TEST] MockPlatform: next_message waiting...");
            let msg = self.message_receiver.recv().await;
            eprintln!("[TEST] MockPlatform: next_message received: {:?}", msg);
            msg
        }

        async fn disconnect(&mut self) -> Result<(), Self::Error> {
            eprintln!("[TEST] MockPlatform: disconnect");
            self.is_connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            eprintln!("[TEST] MockPlatform: is_connected -> {}", self.is_connected);
            self.is_connected
        }

        fn platform_name(&self) -> &str {
            "mock"
        }

        async fn get_channel_emotes(&self, _channel: &str) -> Result<Vec<Emote>, Self::Error> {
            Ok(vec![])
        }

        async fn get_global_emotes(&self) -> Result<Vec<Emote>, Self::Error> {
            Ok(vec![])
        }

        fn parse_emotes(&self, _content: &str, _emote_data: &str) -> Vec<Emote> {
            vec![]
        }

        fn parse_badges(&self, _badge_data: &str) -> Vec<Badge> {
            vec![]
        }

        fn apply_message_filters(
            &self,
            _message: &mut ChatMessage,
            _filters: &MessageFilters,
        ) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_platform_manager_run_platform() {
        let mut manager = PlatformManager::new();
        let handle = MockPlatformHandle::new();
        let mock_platform = handle.platform;
        let message_sender = handle.message_sender;

        manager
            .run_platform(
                "mock".to_string(),
                mock_platform,
                "test_channel".to_string(),
            )
            .await
            .unwrap();

        // Give it a moment to connect and join channel
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let test_message = ChatMessage {
            id: "1".to_string(),
            platform: "".to_string(),
            channel: "".to_string(),
            username: "test_user".to_string(),
            content: "Hello, world!".to_string(),
            display_name: None,
            emotes: vec![],
            badges: vec![],
            timestamp: system_time_now(),
            user_color: None,
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

        eprintln!(
            "[TEST] Sending test message via mock sender: {:?}",
            test_message
        );
        message_sender.send(test_message.clone()).unwrap();

        eprintln!("[TEST] Waiting for message from manager...");
        let received_message = manager.next_message().await;
        eprintln!(
            "[TEST] Received message from manager: {:?}",
            received_message
        );

        assert!(received_message.is_some());
        let received_message = received_message.unwrap();

        assert_eq!(received_message.id, test_message.id);
        assert_eq!(received_message.content, test_message.content);
        assert_eq!(received_message.platform, "mock");
        assert_eq!(received_message.channel, "test_channel");
    }

    #[tokio::test]
    async fn test_connection_management() {
        let mut manager = PlatformManager::new();

        // Test adding connections
        let conn1 = ConnectionInfo {
            id: "conn1".to_string(),
            platform: "twitch".to_string(),
            channel: "channel1".to_string(),
            enabled: true,
            display_name: Some("Connection 1".to_string()),
        };

        let conn2 = ConnectionInfo {
            id: "conn2".to_string(),
            platform: "youtube".to_string(),
            channel: "channel2".to_string(),
            enabled: false,
            display_name: Some("Connection 2".to_string()),
        };

        manager.add_connection(conn1.clone());
        manager.add_connection(conn2.clone());

        assert_eq!(manager.get_connections().len(), 2);
        assert_eq!(manager.get_enabled_connections().len(), 1);

        // Test platform registration
        let handle = MockPlatformHandle::new();
        let mock_platform = handle.platform;
        manager.register_platform("twitch".to_string(), Box::new(mock_platform));

        assert!(manager.get_platform_names().contains(&"twitch".to_string()));
    }

    #[tokio::test]
    async fn test_message_flow_with_multiple_messages() {
        let mut manager = PlatformManager::new();
        let handle = MockPlatformHandle::new();
        let mock_platform = handle.platform;
        let message_sender = handle.message_sender;

        manager
            .run_platform(
                "mock".to_string(),
                mock_platform,
                "test_channel".to_string(),
            )
            .await
            .unwrap();

        // Send multiple test messages
        let messages = vec!["First message", "Second message", "Third message"];

        for (i, content) in messages.iter().enumerate() {
            let test_message = ChatMessage {
                id: format!("{}", i),
                platform: "".to_string(),
                channel: "".to_string(),
                username: format!("user{}", i),
                content: content.to_string(),
                display_name: None,
                emotes: vec![],
                badges: vec![],
                timestamp: system_time_now(),
                user_color: None,
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

            eprintln!("[TEST] Sending message {}: {}", i, content);
            message_sender.send(test_message).unwrap();
        }

        // Receive and verify all messages
        for expected_content in messages {
            if let Some(received_message) = manager.next_message().await {
                eprintln!("[TEST] Received: {}", received_message.content);
                assert_eq!(received_message.content, expected_content);
                assert_eq!(received_message.platform, "mock");
                assert_eq!(received_message.channel, "test_channel");
            } else {
                panic!("Expected message not received: {}", expected_content);
            }
        }
    }

    #[tokio::test]
    async fn test_connection_disabled() {
        let mut manager = PlatformManager::new();

        let disabled_conn = ConnectionInfo {
            id: "disabled_conn".to_string(),
            platform: "twitch".to_string(),
            channel: "channel1".to_string(),
            enabled: false,
            display_name: Some("Disabled Connection".to_string()),
        };

        manager.add_connection(disabled_conn);

        // Attempt to start disabled connection should fail
        let result = manager.start_connection("disabled_conn").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }
}
