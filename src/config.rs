use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub platform: PlatformConfig,
    pub window: WindowConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlatformConfig {
    #[serde(rename = "type")]
    pub platform_type: String,
    pub default_channel: String,
    pub username: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WindowConfig {
    pub message_duration_seconds: u64,
    pub max_windows: usize,
    pub test_message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DisplayConfig {
    pub monitor_margin: i32,
    pub window_size: i32,
    pub grid_size: i32,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileError(e.to_string()))?;
        
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        Ok(config)
    }
    
    pub fn load_default() -> Result<Self, ConfigError> {
        Self::load_from_file("config.json")
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;
        
        fs::write(path, content)
            .map_err(|e| ConfigError::FileError(e.to_string()))?;
        
        Ok(())
    }
    
    pub fn message_duration(&self) -> Duration {
        Duration::from_secs(self.window.message_duration_seconds)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            platform: PlatformConfig {
                platform_type: "twitch".to_string(),
                default_channel: "apika_luca".to_string(),
                username: "USERNAME".to_string(),
            },
            window: WindowConfig {
                message_duration_seconds: 10,
                max_windows: 100,
                test_message: "TEST".to_string(),
            },
            display: DisplayConfig {
                monitor_margin: 40,
                window_size: 200,
                grid_size: 100,
            },
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileError(String),
    ParseError(String),
    SerializeError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileError(msg) => write!(f, "File error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}