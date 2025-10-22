use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub platforms: HashMap<String, PlatformConfig>,
    pub connections: Vec<ConnectionConfig>,
    pub window: WindowConfig,
    pub display: DisplayConfig,
    pub emotes: EmoteConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlatformConfig {
    pub platform_type: PlatformType,
    pub enabled: bool,
    pub credentials: Credentials,
    pub settings: PlatformSettings,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            platform_type: PlatformType::Twitch,
            enabled: true,
            credentials: Credentials::default(),
            settings: PlatformSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    Twitch,
    YouTube,
    Kick,
    Trovo,
    Facebook,
}

impl std::fmt::Display for PlatformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformType::Twitch => write!(f, "twitch"),
            PlatformType::YouTube => write!(f, "youtube"),
            PlatformType::Kick => write!(f, "kick"),
            PlatformType::Trovo => write!(f, "trovo"),
            PlatformType::Facebook => write!(f, "facebook"),
        }
    }
}

impl Default for PlatformType {
    fn default() -> Self {
        PlatformType::Twitch
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Credentials {
    pub username: Option<String>,
    pub oauth_token: Option<String>,
    pub api_key: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl Default for Credentials {
    fn default() -> Self {
        Self {
            username: None,
            oauth_token: None,
            api_key: None,
            client_id: None,
            client_secret: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlatformSettings {
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub message_buffer_size: usize,
    pub enable_emotes: bool,
    pub enable_badges: bool,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl Default for PlatformSettings {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
            message_buffer_size: 1000,
            enable_emotes: true,
            enable_badges: true,
            custom_settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectionConfig {
    pub id: String,
    pub platform: String,
    pub channel: String,
    pub enabled: bool,
    pub filters: MessageFilters,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MessageFilters {
    pub min_message_length: Option<usize>,
    pub max_message_length: Option<usize>,
    pub blocked_users: Vec<String>,
    pub allowed_users: Vec<String>,
    pub blocked_words: Vec<String>,
    pub commands_only: bool,
    pub subscribers_only: bool,
    pub vip_only: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WindowConfig {
    pub message_duration_seconds: u64,
    pub max_windows: usize,
    pub test_message: String,
    pub animation_enabled: bool,
    pub fade_in_duration_ms: u64,
    pub fade_out_duration_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DisplayConfig {
    pub monitor_margin: i32,
    pub window_size: i32,
    pub grid_size: i32,
    pub font_family: String,
    pub font_size: u32,
    pub background_color: String,
    pub text_color: String,
    pub username_color: String,
    pub border_radius: u32,
    pub opacity: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmoteConfig {
    pub enable_global_emotes: bool,
    pub enable_channel_emotes: bool,
    pub enable_subscriber_emotes: bool,
    pub enable_bttv: bool,
    pub enable_ffz: bool,
    pub enable_7tv: bool,
    pub emote_size: EmoteSize,
    pub emote_animation: bool,
    pub max_emotes_per_message: usize,
    pub cache_enabled: bool,
    pub cache_ttl_hours: u64,
}

impl Default for EmoteConfig {
    fn default() -> Self {
        Self {
            enable_global_emotes: true,
            enable_channel_emotes: true,
            enable_subscriber_emotes: true,
            enable_bttv: true,
            enable_ffz: true,
            enable_7tv: true,
            emote_size: EmoteSize::Medium,
            emote_animation: true,
            max_emotes_per_message: 50,
            cache_enabled: true,
            cache_ttl_hours: 24,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum EmoteSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl Default for EmoteSize {
    fn default() -> Self {
        EmoteSize::Medium
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file_enabled: bool,
    pub console_enabled: bool,
    pub log_file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content =
            fs::read_to_string(path).map_err(|e| ConfigError::FileError(e.to_string()))?;

        let config: Config =
            serde_json::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;

        Ok(config)
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        Self::load_with_fallback("config.json")
    }

    pub fn load_with_fallback<P: AsRef<Path>>(external_path: P) -> Result<Self, ConfigError> {
        // Intentar cargar configuración externa
        match Self::load_from_file(&external_path) {
            Ok(config) => {
                println!("[CONFIG] ✅ External config loaded from: {:?}", external_path.as_ref());
                Self::log_loaded_config(&config);
                Ok(config)
            },
            Err(e) => {
                println!("[CONFIG] ⚠️ Could not load external config from {:?}: {}", external_path.as_ref(), e);
                println!("[CONFIG] 🔄 Creating default config file...");

                // Si el archivo externo no existe o hay error, crearlo con valores por defecto
                let default_config = Self::default();
                if let Err(e) = default_config.save_to_file(&external_path) {
                    eprintln!("[CONFIG] ❌ Warning: Could not create external config file: {}", e);
                } else {
                    println!("[CONFIG] ✅ Default config saved to: {:?}", external_path.as_ref());
                }

                Self::log_loaded_config(&default_config);
                Ok(default_config)
            }
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        self.validate()?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        fs::write(path, content).map_err(|e| ConfigError::FileError(e.to_string()))?;

        Ok(())
    }

    pub fn message_duration(&self) -> Duration {
        Duration::from_secs(self.window.message_duration_seconds)
    }

    pub fn get_enabled_platforms(&self) -> Vec<&str> {
        self.platforms
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub fn get_enabled_connections(&self) -> Vec<&ConnectionConfig> {
        self.connections
            .iter()
            .filter(|conn| conn.enabled)
            .collect()
    }

    pub fn get_platform_config(&self, platform_name: &str) -> Option<&PlatformConfig> {
        self.platforms.get(platform_name)
    }

    /// Log the loaded configuration for debugging purposes
    fn log_loaded_config(config: &Config) {
        println!("[CONFIG] 📊 Configuration Summary:");
        println!("[CONFIG]   Platforms: {} ({} enabled)",
                 config.platforms.len(),
                 config.get_enabled_platforms().len());
        println!("[CONFIG]   Connections: {} ({} enabled)",
                 config.connections.len(),
                 config.get_enabled_connections().len());

        for platform_name in config.get_enabled_platforms() {
            if let Some(platform_config) = config.get_platform_config(platform_name) {
                println!("[CONFIG]     - {}: enabled={}, credentials={}",
                         platform_name,
                         platform_config.enabled,
                         if platform_config.credentials.username.is_some() { "set" } else { "none" });
            }
        }

        for conn in config.get_enabled_connections() {
            println!("[CONFIG]     - Connection '{}' -> {} @ channel '{}'",
                     conn.id, conn.platform, conn.channel);
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Validar que haya al menos una plataforma habilitada
        if self.platforms.iter().all(|(_, config)| !config.enabled) {
            return Err(ConfigError::ValidationError(
                "At least one platform must be enabled".to_string(),
            ));
        }

        // Validar que haya al menos una conexión habilitada
        if self.connections.iter().all(|conn| !conn.enabled) {
            return Err(ConfigError::ValidationError(
                "At least one connection must be enabled".to_string(),
            ));
        }

        // Validar que todas las conexiones referenciadas existan en plataformas
        for conn in &self.connections {
            if !self.platforms.contains_key(&conn.platform) {
                return Err(ConfigError::ValidationError(format!(
                    "Connection '{}' references non-existent platform '{}'",
                    conn.id, conn.platform
                )));
            }
        }

        // Validar configuraciones de ventana
        if self.window.message_duration_seconds == 0 {
            return Err(ConfigError::ValidationError(
                "message_duration_seconds must be greater than 0".to_string(),
            ));
        }

        if self.window.max_windows == 0 {
            return Err(ConfigError::ValidationError(
                "max_windows must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut platforms = HashMap::new();

        // Configuración por defecto para Twitch
        platforms.insert(
            "twitch".to_string(),
            PlatformConfig {
                platform_type: PlatformType::Twitch,
                enabled: true,
                credentials: Credentials {
                    username: None,
                    oauth_token: None,
                    api_key: None,
                    client_id: None,
                    client_secret: None,
                },
                settings: PlatformSettings {
                    max_reconnect_attempts: 5,
                    reconnect_delay_ms: 5000,
                    message_buffer_size: 1000,
                    enable_emotes: true,
                    enable_badges: true,
                    custom_settings: HashMap::new(),
                },
            },
        );

        // Configuración por defecto para Kick
        platforms.insert(
            "kick".to_string(),
            PlatformConfig {
                platform_type: PlatformType::Kick,
                enabled: true,
                credentials: Credentials::default(),
                settings: PlatformSettings {
                    max_reconnect_attempts: 5,
                    reconnect_delay_ms: 5000,
                    message_buffer_size: 1000,
                    enable_emotes: true,
                    enable_badges: true,
                    custom_settings: HashMap::new(),
                },
            },
        );

        Self {
            platforms,
            connections: vec![
                ConnectionConfig {
                    id: "twitch_main".to_string(),
                    platform: "twitch".to_string(),
                    channel: "gohuntleo".to_string(),
                    enabled: false, // Deshabilitado por defecto para evitar errores si no hay token
                    filters: MessageFilters {
                        min_message_length: None,
                        max_message_length: Some(500),
                        blocked_users: vec![],
                        allowed_users: vec![],
                        blocked_words: vec![],
                        commands_only: false,
                        subscribers_only: false,
                        vip_only: false,
                    },
                    display_name: Some("Main Twitch Chat".to_string()),
                },
                ConnectionConfig {
                    id: "kick_main".to_string(),
                    platform: "kick".to_string(),
                    channel: "spreen".to_string(),
                    enabled: true, // Habilitado por defecto, Kick no requiere autenticación
                    filters: MessageFilters {
                        min_message_length: None,
                        max_message_length: Some(500),
                        blocked_users: vec![],
                        allowed_users: vec![],
                        blocked_words: vec![],
                        commands_only: false,
                        subscribers_only: false,
                        vip_only: false,
                    },
                    display_name: Some("Kick Chat".to_string()),
                },
            ],
            window: WindowConfig {
                message_duration_seconds: 10,
                max_windows: 100,
                test_message: "TEST".to_string(),
                animation_enabled: true,
                fade_in_duration_ms: 300,
                fade_out_duration_ms: 500,
            },
            display: DisplayConfig {
                monitor_margin: 40,
                window_size: 200,
                grid_size: 100,
                font_family: "Arial".to_string(),
                font_size: 14,
                background_color: "#1e1e1e".to_string(),
                text_color: "#ffffff".to_string(),
                username_color: "#00ff00".to_string(),
                border_radius: 8,
                opacity: 0.9,
            },
            emotes: EmoteConfig {
                enable_global_emotes: true,
                enable_channel_emotes: true,
                enable_subscriber_emotes: true,
                enable_bttv: true,
                enable_ffz: true,
                enable_7tv: true,
                emote_size: EmoteSize::Medium,
                emote_animation: true,
                max_emotes_per_message: 50,
                cache_enabled: true,
                cache_ttl_hours: 24,
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                file_enabled: true,
                console_enabled: true,
                log_file_path: Some("overlay.log".to_string()),
                max_file_size_mb: 10,
                max_files: 5,
            },
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileError(String),
    ParseError(String),
    SerializeError(String),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileError(msg) => write!(f, "File error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
