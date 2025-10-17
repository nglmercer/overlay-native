use crate::connection::{Emote, EmoteMetadata, EmoteSource, TextPosition};
use std::collections::HashMap;
use std::path::PathBuf;

/// Renderer de emotes que maneja la obtención y procesamiento de imágenes
pub struct EmoteRenderer {
    cache_dir: PathBuf,
    max_cache_size_mb: u64,
    supported_formats: Vec<String>,
    scaling_factor: f32,
    default_size: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct RenderedEmote {
    pub emote: Emote,
    pub local_path: Option<PathBuf>,
    pub data: Option<Vec<u8>>,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub render_time: std::time::Duration,
}

#[derive(Debug)]
pub enum RenderError {
    NetworkError(String),
    IoError(String),
    FormatError(String),
    CacheError(String),
    SizeError(String),
}

impl EmoteRenderer {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            max_cache_size_mb: 100,
            supported_formats: vec!["png".to_string(), "gif".to_string(), "webp".to_string()],
            scaling_factor: 1.0,
            default_size: (32, 32),
        }
    }

    /// Renderiza un emote obteniendo su imagen y procesándola
    pub async fn render_emote(&self, emote: &Emote) -> Result<RenderedEmote, RenderError> {
        let start_time = std::time::Instant::now();

        // Determinar URL del emote
        let url = self.resolve_emote_url(emote)?;

        // Obtener la imagen
        let image_data = self.fetch_emote_image(&url).await?;

        // Determinar formato
        let format = self.detect_image_format(&image_data)?;

        // Procesar imagen (escalar si es necesario)
        let processed_data = self.process_image(&image_data, emote).await?;

        // Guardar en cache si corresponde
        let local_path = self.cache_emote(emote, &processed_data).await?;

        let render_time = start_time.elapsed();

        Ok(RenderedEmote {
            emote: emote.clone(),
            local_path,
            data: Some(processed_data),
            format,
            width: emote.width.unwrap_or(self.default_size.0),
            height: emote.height.unwrap_or(self.default_size.1),
            file_size: image_data.len() as u64,
            render_time,
        })
    }

    /// Renderiza múltiples emotes en lote
    pub async fn render_emotes_batch(
        &self,
        emotes: &[Emote],
    ) -> Vec<Result<RenderedEmote, RenderError>> {
        let mut results = Vec::new();

        // Usar futures para procesamiento concurrente
        let futures: Vec<_> = emotes
            .iter()
            .map(|emote| self.render_emote(emote))
            .collect();

        let batch_results = futures::future::join_all(futures).await;

        for result in batch_results {
            results.push(result);
        }

        results
    }

    /// Resuelve la URL de un emote basado en su source
    pub fn resolve_emote_url(&self, emote: &Emote) -> Result<String, RenderError> {
        if let Some(url) = &emote.url {
            return Ok(url.clone());
        }

        // Construir URL basada en el source
        match emote.source {
            EmoteSource::Twitch | EmoteSource::TwitchGlobal | EmoteSource::TwitchSubscriber => {
                Ok(format!(
                    "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/1.0",
                    emote.id
                ))
            }
            EmoteSource::BTTV => Ok(format!("https://cdn.betterttv.net/emote/{}/3x", emote.id)),
            EmoteSource::FFZ => Ok(format!("https://cdn.frankerfacez.com/emote/{}/4", emote.id)),
            EmoteSource::SevenTV => Ok(format!("https://cdn.7tv.app/emote/{}/4x", emote.id)),
            _ => Err(RenderError::FormatError(
                "Cannot determine URL for emote source".to_string(),
            )),
        }
    }

    /// Obtiene la imagen de un emote desde la URL
    async fn fetch_emote_image(&self, url: &str) -> Result<Vec<u8>, RenderError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Overlay-Native/1.0")
            .build()
            .map_err(|e| RenderError::NetworkError(e.to_string()))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| RenderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RenderError::NetworkError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        response
            .bytes()
            .await
            .map_err(|e| RenderError::NetworkError(e.to_string()))
            .map(|bytes| bytes.to_vec())
    }

    /// Detecta el formato de imagen
    pub fn detect_image_format(&self, data: &[u8]) -> Result<String, RenderError> {
        if data.len() < 8 {
            return Err(RenderError::FormatError("File too small".to_string()));
        }

        // Detectar por magic bytes
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            Ok("png".to_string())
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            Ok("gif".to_string())
        } else if data.starts_with(&[0x52, 0x49, 0x46, 0x46]) && data.len() > 12 {
            // WebP/RIFF
            if &data[8..12] == b"WEBP" {
                Ok("webp".to_string())
            } else {
                Ok("riff".to_string())
            }
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Ok("jpg".to_string())
        } else {
            // Intentar detectar por extensión si no se puede por magic bytes
            Err(RenderError::FormatError("Unknown image format".to_string()))
        }
    }

    /// Procesa una imagen (escalado, optimización, etc.)
    async fn process_image(&self, data: &[u8], emote: &Emote) -> Result<Vec<u8>, RenderError> {
        // Para una implementación básica, simplemente devolvemos los datos originales
        // En una implementación completa, aquí se usaría una librería de procesamiento de imágenes
        // como image-rs para escalar, optimizar, etc.

        // Verificar límites de tamaño
        if data.len() > 10 * 1024 * 1024 {
            // 10MB limit
            return Err(RenderError::SizeError("Image too large".to_string()));
        }

        // Verificar dimensiones si están disponibles
        if let (Some(width), Some(height)) = (emote.width, emote.height) {
            if width > 1024 || height > 1024 {
                return Err(RenderError::SizeError(
                    "Image dimensions too large".to_string(),
                ));
            }
        }

        Ok(data.to_vec())
    }

    /// Guarda un emote en el cache local
    async fn cache_emote(
        &self,
        emote: &Emote,
        data: &[u8],
    ) -> Result<Option<PathBuf>, RenderError> {
        // Crear directorio de cache si no existe
        if !self.cache_dir.exists() {
            tokio::fs::create_dir_all(&self.cache_dir)
                .await
                .map_err(|e| RenderError::IoError(e.to_string()))?;
        }

        // Generar nombre de archivo único
        let file_name = format!("{}_{}.{}", emote.source, emote.id, "png");
        let file_path = self.cache_dir.join(file_name);

        // Verificar si ya existe
        if file_path.exists() {
            return Ok(Some(file_path));
        }

        // Guardar archivo
        tokio::fs::write(&file_path, data)
            .await
            .map_err(|e| RenderError::IoError(e.to_string()))?;

        Ok(Some(file_path))
    }

    /// Limpia el cache de emotes
    pub async fn clean_cache(&self) -> Result<u64, RenderError> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        let mut entries = tokio::fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| RenderError::IoError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| RenderError::IoError(e.to_string()))?
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    total_size += metadata.len();

                    // Eliminar si es demasiado grande
                    if total_size > self.max_cache_size_mb * 1024 * 1024 {
                        tokio::fs::remove_file(&path)
                            .await
                            .map_err(|e| RenderError::IoError(e.to_string()))?;
                    }
                }
            }
        }

        Ok(total_size)
    }

    /// Obtiene estadísticas del cache
    pub async fn get_cache_stats(&self) -> Result<CacheStats, RenderError> {
        if !self.cache_dir.exists() {
            return Ok(CacheStats {
                file_count: 0,
                total_size_bytes: 0,
                total_size_mb: 0.0,
                oldest_file: None,
                newest_file: None,
            });
        }

        let mut file_count = 0u64;
        let mut total_size = 0u64;
        let mut oldest_time = std::time::SystemTime::now();
        let mut newest_time = std::time::UNIX_EPOCH;
        let mut oldest_file = None;
        let mut newest_file = None;

        let mut entries = tokio::fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| RenderError::IoError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| RenderError::IoError(e.to_string()))?
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    file_count += 1;
                    total_size += metadata.len();

                    if let Ok(modified) = metadata.modified() {
                        if modified < oldest_time {
                            oldest_time = modified;
                            oldest_file = Some(
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                            );
                        }
                        if modified > newest_time {
                            newest_time = modified;
                            newest_file = Some(
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }

        Ok(CacheStats {
            file_count,
            total_size_bytes: total_size,
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
            oldest_file,
            newest_file,
        })
    }

    /// Configura el factor de escalado
    pub fn set_scaling_factor(&mut self, factor: f32) {
        self.scaling_factor = factor.clamp(0.1, 5.0);
    }

    /// Configura el tamaño por defecto
    pub fn set_default_size(&mut self, width: u32, height: u32) {
        self.default_size = (width.clamp(8, 512), height.clamp(8, 512));
    }

    /// Configura el tamaño máximo del cache
    pub fn set_max_cache_size(&mut self, size_mb: u64) {
        self.max_cache_size_mb = size_mb.clamp(1, 1000);
    }
}

/// Estadísticas del cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub file_count: u64,
    pub total_size_bytes: u64,
    pub total_size_mb: f64,
    pub oldest_file: Option<String>,
    pub newest_file: Option<String>,
}

impl Default for EmoteRenderer {
    fn default() -> Self {
        Self::new(std::env::temp_dir().join("overlay-native").join("emotes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{EmoteMetadata, TextPosition};
    use tempfile::TempDir;

    fn create_test_emote(id: &str, name: &str, source: EmoteSource) -> Emote {
        Emote {
            id: id.to_string(),
            name: name.to_string(),
            source,
            positions: vec![TextPosition {
                start: 0,
                end: name.len() - 1,
            }],
            url: None,
            is_animated: false,
            width: Some(32),
            height: Some(32),
            metadata: EmoteMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_resolve_emote_url() {
        let renderer = EmoteRenderer::new(PathBuf::from("/tmp"));

        let twitch_emote = create_test_emote("25", "Kappa", EmoteSource::Twitch);
        let url = renderer.resolve_emote_url(&twitch_emote).unwrap();
        assert_eq!(
            url,
            "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/1.0"
        );

        let bttv_emote =
            create_test_emote("5e7c3560b4d743c5830f0ae4", "FeelsBadMan", EmoteSource::BTTV);
        let url = renderer.resolve_emote_url(&bttv_emote).unwrap();
        assert_eq!(
            url,
            "https://cdn.betterttv.net/emote/5e7c3560b4d743c5830f0ae4/3x"
        );
    }

    #[test]
    fn test_detect_image_format() {
        let renderer = EmoteRenderer::new(PathBuf::from("/tmp"));

        // PNG magic bytes
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(renderer.detect_image_format(&png_data).unwrap(), "png");

        // GIF magic bytes (need at least 8 bytes)
        let gif_data = b"GIF89a__".to_vec();
        assert_eq!(renderer.detect_image_format(&gif_data).unwrap(), "gif");
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = EmoteRenderer::new(temp_dir.path().to_path_buf());

        let stats = renderer.get_cache_stats().await.unwrap();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }
}
