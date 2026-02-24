use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub transport: TransportConfig,
    pub window: WindowConfig,
    pub display: DisplayConfig,
    pub logging: LoggingConfig,
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
    pub templates_dir: Option<String>,
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

/// Transport layer configuration (IPC/WebSocket)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransportConfig {
    pub websocket_enabled: bool,
    pub websocket_bind: String,
    pub ipc_enabled: bool,
    pub ipc_socket_path: String,
    pub max_connections: usize,
    pub strict_validation: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            websocket_enabled: true,
            websocket_bind: "127.0.0.1:9001".to_string(),
            ipc_enabled: true,
            ipc_socket_path: "/tmp/overlay-native.sock".to_string(),
            max_connections: 100,
            strict_validation: true,
        }
    }
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
        let content = fs::read_to_string(path).map_err(|e| ConfigError::File(e.to_string()))?;
        let config: Config =
            serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        Self::load_modular().or_else(|_| Self::load_with_fallback("config.json"))
    }

    pub fn load_modular() -> Result<Self, ConfigError> {
        let config_dir = Path::new("config");
        if !config_dir.exists() {
            return Err(ConfigError::File("Config directory not found".into()));
        }

        let mut config = Self::default();

        if let Ok(transport) =
            Self::load_part::<TransportConfig, _>(config_dir.join("transport.json"))
        {
            config.transport = transport;
            println!("[CONFIG] Loaded transport.json");
        }
        if let Ok(window) = Self::load_part::<WindowConfig, _>(config_dir.join("window.json")) {
            config.window = window;
            println!("[CONFIG] Loaded window.json");
        }
        if let Ok(display) = Self::load_part::<DisplayConfig, _>(config_dir.join("display.json")) {
            config.display = display;
            println!("[CONFIG] Loaded display.json");
        }
        if let Ok(logging) = Self::load_part::<LoggingConfig, _>(config_dir.join("logging.json")) {
            config.logging = logging;
            println!("[CONFIG] Loaded logging.json");
        }

        config.validate()?;
        Ok(config)
    }

    fn load_part<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(path: P) -> Result<T, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::File(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    pub fn load_with_fallback<P: AsRef<Path>>(external_path: P) -> Result<Self, ConfigError> {
        match Self::load_from_file(&external_path) {
            Ok(config) => {
                println!(
                    "[CONFIG] ✅ External config loaded from: {:?}",
                    external_path.as_ref()
                );
                Ok(config)
            }
            Err(_) => {
                println!("[CONFIG] 🔄 Creating default config file...");
                let default_config = Self::default();
                if let Err(e) = default_config.save_to_file(&external_path) {
                    eprintln!(
                        "[CONFIG] ❌ Warning: Could not create external config file: {}",
                        e
                    );
                }
                Ok(default_config)
            }
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        self.validate()?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        fs::write(path, content).map_err(|e| ConfigError::File(e.to_string()))?;
        Ok(())
    }

    pub fn message_duration(&self) -> Duration {
        Duration::from_secs(self.window.message_duration_seconds)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.window.message_duration_seconds == 0 {
            return Err(ConfigError::Validation(
                "message_duration_seconds must be greater than 0".to_string(),
            ));
        }
        if self.window.max_windows == 0 {
            return Err(ConfigError::Validation(
                "max_windows must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
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
                templates_dir: Some("templates".to_string()),
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
    File(String),
    Parse(String),
    Serialize(String),
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::File(msg) => write!(f, "File error: {}", msg),
            ConfigError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::Serialize(msg) => write!(f, "Serialize error: {}", msg),
            ConfigError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}
