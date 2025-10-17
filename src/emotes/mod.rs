pub mod cache;
pub mod parser;
pub mod providers;
pub mod renderer;

pub use cache::*;
pub use parser::*;
pub use providers::*;
pub use renderer::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sistema unificado de manejo de emotes para todas las plataformas
pub struct EmoteSystem {
    pub cache: EmoteCache,
    providers: HashMap<String, Box<dyn EmoteProvider>>,
    parser: EmoteParser,
    renderer: EmoteRenderer,
    config: crate::config::EmoteConfig,
}

impl EmoteSystem {
    pub fn new(config: crate::config::EmoteConfig) -> Self {
        let mut providers: HashMap<String, Box<dyn EmoteProvider>> = HashMap::new();

        // Registrar proveedores por defecto
        providers.insert("twitch".to_string(), Box::new(TwitchEmoteProvider::new()));
        providers.insert("bttv".to_string(), Box::new(BTTVEmoteProvider::new()));
        providers.insert("ffz".to_string(), Box::new(FFZEmoteProvider::new()));
        providers.insert("7tv".to_string(), Box::new(SevenTVEmoteProvider::new()));

        Self {
            cache: EmoteCache::new(config.cache_ttl_hours),
            providers,
            parser: EmoteParser::new(),
            renderer: EmoteRenderer::new(
                std::env::temp_dir().join("overlay-native").join("emotes"),
            ),
            config,
        }
    }

    /// Registra un nuevo proveedor de emotes
    pub fn register_provider(&mut self, name: String, provider: Box<dyn EmoteProvider>) {
        self.providers.insert(name, provider);
    }

    /// Parsea emotes en un mensaje de chat
    pub async fn parse_message_emotes(
        &mut self,
        message: &str,
        platform: &str,
        channel: &str,
        raw_emote_data: &str,
    ) -> Result<Vec<crate::connection::Emote>, EmoteError> {
        let mut emotes = Vec::new();

        // Obtener emotes del parser específico de la plataforma
        if let Some(provider) = self.providers.get(platform) {
            match provider.parse_emotes(message, raw_emote_data).await {
                Ok(platform_emotes) => {
                    for mut emote in platform_emotes {
                        // Enriquecer con datos del cache si es necesario
                        if let Some(cached) = self.cache.get(&emote.id) {
                            emote.url = cached.url.clone();
                            emote.is_animated = cached.is_animated;
                            emote.width = cached.width;
                            emote.height = cached.height;
                        }

                        emotes.push(emote);
                    }
                }
                Err(_) => {
                    // Handle provider errors gracefully - continue with empty emotes
                    // This allows the system to remain functional even when providers fail
                }
            }
        }

        // Buscar emotes de terceros (BTTV, FFZ, 7TV) si está habilitado
        if self.config.enable_bttv || self.config.enable_ffz || self.config.enable_7tv {
            let third_party_emotes = self
                .parse_third_party_emotes(message, platform, channel)
                .await?;
            emotes.extend(third_party_emotes);
        }

        // Limitar número de emotes por mensaje
        if emotes.len() > self.config.max_emotes_per_message {
            emotes.truncate(self.config.max_emotes_per_message);
        }

        Ok(emotes)
    }

    /// Parsea emotes de terceros (BTTV, FFZ, 7TV)
    async fn parse_third_party_emotes(
        &mut self,
        message: &str,
        platform: &str,
        channel: &str,
    ) -> Result<Vec<crate::connection::Emote>, EmoteError> {
        let mut emotes = Vec::new();
        let known_emotes = self.get_known_third_party_emotes(platform, channel).await?;

        for (provider_name, provider_emotes) in known_emotes {
            for emote_data in provider_emotes {
                let positions = self.parser.find_emote_positions(message, &emote_data.name);

                if !positions.is_empty() {
                    emotes.push(crate::connection::Emote {
                        id: emote_data.id,
                        name: emote_data.name,
                        source: self.map_provider_to_source(&provider_name),
                        positions,
                        url: emote_data.url,
                        is_animated: emote_data.is_animated,
                        width: emote_data.width,
                        height: emote_data.height,
                        metadata: crate::connection::EmoteMetadata {
                            is_zero_width: emote_data.is_zero_width,
                            modifier: emote_data.modifier,
                            emote_set_id: emote_data.emote_set_id,
                            tier: None,
                        },
                    });
                }
            }
        }

        Ok(emotes)
    }

    /// Obtiene emotes conocidos de terceros para un canal
    async fn get_known_third_party_emotes(
        &mut self,
        platform: &str,
        channel: &str,
    ) -> Result<HashMap<String, Vec<EmoteData>>, EmoteError> {
        let mut result = HashMap::new();

        if self.config.enable_bttv {
            if let Some(provider) = self.providers.get("bttv") {
                let emotes = provider.get_channel_emotes(platform, channel).await?;
                result.insert("bttv".to_string(), emotes);
            }
        }

        if self.config.enable_ffz {
            if let Some(provider) = self.providers.get("ffz") {
                let emotes = provider.get_channel_emotes(platform, channel).await?;
                result.insert("ffz".to_string(), emotes);
            }
        }

        if self.config.enable_7tv {
            if let Some(provider) = self.providers.get("7tv") {
                let emotes = provider.get_channel_emotes(platform, channel).await?;
                result.insert("7tv".to_string(), emotes);
            }
        }

        Ok(result)
    }

    /// Mapea nombre de proveedor a source de emote
    fn map_provider_to_source(&self, provider: &str) -> crate::connection::EmoteSource {
        match provider {
            "twitch" => crate::connection::EmoteSource::Twitch,
            "bttv" => crate::connection::EmoteSource::BTTV,
            "ffz" => crate::connection::EmoteSource::FFZ,
            "7tv" => crate::connection::EmoteSource::SevenTV,
            "youtube" => crate::connection::EmoteSource::YouTube,
            _ => crate::connection::EmoteSource::Local,
        }
    }

    /// Precarga emotes globales
    pub async fn preload_global_emotes(&mut self) -> Result<(), EmoteError> {
        let mut total_emotes = 0;
        let mut failed_providers = Vec::new();

        for (name, provider) in &self.providers {
            println!("   📥 Loading {} global emotes...", name);

            match provider.get_global_emotes().await {
                Ok(global_emotes) => {
                    let count = global_emotes.len();

                    for emote_data in global_emotes {
                        if self.config.cache_enabled {
                            self.cache.insert(
                                emote_data.id.clone(),
                                crate::connection::Emote {
                                    id: emote_data.id.clone(),
                                    name: emote_data.name.clone(),
                                    source: self.map_provider_to_source(name),
                                    positions: Vec::new(),
                                    url: emote_data.url.clone(),
                                    is_animated: emote_data.is_animated,
                                    width: emote_data.width,
                                    height: emote_data.height,
                                    metadata: crate::connection::EmoteMetadata {
                                        is_zero_width: emote_data.is_zero_width,
                                        modifier: emote_data.modifier,
                                        emote_set_id: emote_data.emote_set_id,
                                        tier: None,
                                    },
                                },
                            );
                        }
                    }

                    total_emotes += count;
                    println!("   ✅ Loaded {} emotes from {}", count, name);
                }
                Err(e) => {
                    eprintln!("   ⚠️  Failed to load {} emotes: {}", name, e);
                    failed_providers.push((name.clone(), e.to_string()));
                }
            }
        }

        println!("📊 Total emotes loaded: {}", total_emotes);

        if !failed_providers.is_empty() {
            eprintln!("⚠️  Some providers failed:");
            for (provider, error) in &failed_providers {
                eprintln!("   - {}: {}", provider, error);
            }
        }

        Ok(())
    }

    /// Limpia el cache de emotes
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Verifica si el cache está expirado
    pub fn is_cache_expired(&self) -> bool {
        self.cache.is_expired()
    }

    /// Actualiza la configuración
    pub fn update_config(&mut self, config: crate::config::EmoteConfig) {
        self.config = config;
        self.cache = EmoteCache::new(self.config.cache_ttl_hours);
    }
}

/// Trait para proveedores de emotes
#[async_trait::async_trait]
pub trait EmoteProvider: Send + Sync {
    /// Parsea emotes desde datos crudos de la plataforma
    async fn parse_emotes(
        &self,
        message: &str,
        emote_data: &str,
    ) -> Result<Vec<crate::connection::Emote>, EmoteError>;

    /// Obtiene emotes de un canal específico
    async fn get_channel_emotes(
        &self,
        platform: &str,
        channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError>;

    /// Obtiene emotes globales
    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError>;

    /// Nombre del proveedor
    fn provider_name(&self) -> &str;
}

/// Datos de emote para intercambio entre proveedores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmoteData {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub is_animated: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_zero_width: bool,
    pub modifier: bool,
    pub emote_set_id: Option<String>,
}

/// Errores del sistema de emotes
#[derive(Debug, thiserror::Error)]
pub enum EmoteError {
    #[error("Error de parseo: {0}")]
    ParseError(String),

    #[error("Error de red: {0}")]
    NetworkError(String),

    #[error("Error de cache: {0}")]
    CacheError(String),

    #[error("Proveedor no encontrado: {0}")]
    ProviderNotFound(String),

    #[error("Error de API: {0}")]
    ApiError(String),

    #[error("Error de configuración: {0}")]
    ConfigError(String),
}

impl Default for EmoteSystem {
    fn default() -> Self {
        Self::new(crate::config::EmoteConfig::default())
    }
}
