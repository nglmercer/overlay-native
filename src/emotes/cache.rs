use crate::connection::Emote;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cache de emotes con soporte para TTL y limpieza automática
pub struct EmoteCache {
    cache: HashMap<String, CachedEmote>,
    ttl: Duration,
    last_cleanup: Instant,
    cleanup_interval: Duration,
    max_size: usize,
    hit_count: u64,
    miss_count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct CachedEmote {
    emote: Emote,
    #[serde(skip)]
    created_at: Instant,
    #[serde(skip)]
    last_accessed: Instant,
    access_count: u64,
}

impl Default for CachedEmote {
    fn default() -> Self {
        Self {
            emote: Emote::default(),
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
        }
    }
}

impl EmoteCache {
    pub fn new(ttl_hours: u64) -> Self {
        // Handle potential overflow when converting hours to seconds
        let ttl_seconds = ttl_hours.saturating_mul(3600);
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            last_cleanup: Instant::now(),
            cleanup_interval: Duration::from_secs(300), // 5 minutos
            max_size: 10000,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Obtiene un emote del cache
    pub fn get(&mut self, key: &str) -> Option<&Emote> {
        let is_expired = if let Some(cached) = self.cache.get(key) {
            cached.created_at.elapsed() > self.ttl
        } else {
            self.miss_count += 1;
            return None;
        };

        if is_expired {
            self.cache.remove(key);
            self.miss_count += 1;
            return None;
        }

        let cached = self.cache.get_mut(key).unwrap();
        cached.last_accessed = Instant::now();
        cached.access_count += 1;
        self.hit_count += 1;
        Some(&cached.emote)
    }

    /// Inserta un emote en el cache
    pub fn insert(&mut self, key: String, emote: Emote) {
        // Si el cache está lleno, remover el menos usado recientemente
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let now = Instant::now();
        self.cache.insert(
            key,
            CachedEmote {
                emote,
                created_at: now,
                last_accessed: now,
                access_count: 0,
            },
        );
    }

    /// Elimina un emote del cache
    pub fn remove(&mut self, key: &str) -> Option<Emote> {
        self.cache.remove(key).map(|cached| cached.emote)
    }

    /// Verifica si el cache está vacío
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Obtiene el tamaño del cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Verifica si el cache necesita limpieza
    pub fn needs_cleanup(&self) -> bool {
        self.last_cleanup.elapsed() > self.cleanup_interval
    }

    /// Limpia emotes expirados
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (key, cached) in &self.cache {
            if now.duration_since(cached.created_at) > self.ttl {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            self.cache.remove(&key);
        }

        self.last_cleanup = now;
    }

    /// Limpia todo el cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hit_count = 0;
        self.miss_count = 0;
        self.last_cleanup = Instant::now();
    }

    /// Verifica si el cache está expirado
    pub fn is_expired(&self) -> bool {
        self.last_cleanup.elapsed() > self.ttl
    }

    /// Obtiene las estadísticas del cache
    pub fn stats(&mut self) -> CacheStats {
        let stats = CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate: if self.hit_count + self.miss_count > 0 {
                self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
            } else {
                0.0
            },
            ttl_seconds: self.ttl.as_secs(),
            last_cleanup: self.last_cleanup,
        };
        stats
    }

    /// Resetea las estadísticas del cache
    pub fn reset_stats(&mut self) {
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// Obtiene emotes por source
    pub fn get_by_source(&self, source: &crate::connection::EmoteSource) -> Vec<&Emote> {
        self.cache
            .values()
            .filter(|cached| &cached.emote.source == source)
            .map(|cached| &cached.emote)
            .collect()
    }

    /// Obtiene emotes por nombre (búsqueda parcial)
    pub fn search_by_name(&self, query: &str) -> Vec<&Emote> {
        let query_lower = query.to_lowercase();
        self.cache
            .values()
            .filter(|cached| cached.emote.name.to_lowercase().contains(&query_lower))
            .map(|cached| &cached.emote)
            .collect()
    }

    /// Exporta el cache a formato JSON
    pub fn export(&self) -> Result<String, serde_json::Error> {
        let export_data: HashMap<String, &Emote> = self
            .cache
            .iter()
            .map(|(key, cached)| (key.clone(), &cached.emote))
            .collect();
        serde_json::to_string_pretty(&export_data)
    }

    /// Importa emotes desde formato JSON
    pub fn import(&mut self, data: &str) -> Result<(), serde_json::Error> {
        let imported: HashMap<String, Emote> = serde_json::from_str(data)?;
        let now = Instant::now();

        for (key, emote) in imported {
            self.cache.insert(
                key,
                CachedEmote {
                    emote,
                    created_at: now,
                    last_accessed: now,
                    access_count: 0,
                },
            );
        }

        Ok(())
    }

    /// Evicta el least recently used (LRU)
    fn evict_lru(&mut self) {
        if let Some((lru_key, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, cached)| cached.last_accessed)
            .map(|(key, cached)| (key.clone(), cached))
        {
            self.cache.remove(&lru_key);
        }
    }

    /// Configura el tamaño máximo del cache
    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;

        // Si el cache actual excede el nuevo límite, eliminar excedentes
        while self.cache.len() > max_size {
            self.evict_lru();
        }
    }

    /// Configura el TTL
    pub fn set_ttl(&mut self, ttl_hours: u64) {
        self.ttl = Duration::from_secs(ttl_hours * 3600);
    }

    /// Pre-carga emotes populares
    pub async fn preload_popular(&mut self, emotes: Vec<Emote>) {
        for emote in emotes {
            if self.cache.len() < self.max_size {
                self.insert(emote.id.clone(), emote);
            }
        }
    }

    /// Obtiene los emotes más accedidos
    pub fn get_most_accessed(&self, limit: usize) -> Vec<(&String, &Emote, u64)> {
        let mut entries: Vec<_> = self
            .cache
            .iter()
            .map(|(key, cached)| (key, &cached.emote, cached.access_count))
            .collect();

        entries.sort_by(|a, b| b.2.cmp(&a.2));
        entries.into_iter().take(limit).collect()
    }
}

/// Estadísticas del cache
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub ttl_seconds: u64,
    #[serde(skip)]
    pub last_cleanup: Instant,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            size: 0,
            max_size: 1000,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            ttl_seconds: 86400,
            last_cleanup: Instant::now(),
        }
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
    use crate::connection::{EmoteMetadata, EmoteSource, TextPosition};

    fn create_test_emote(id: &str, name: &str) -> Emote {
        Emote {
            id: id.to_string(),
            name: name.to_string(),
            source: EmoteSource::Twitch,
            positions: vec![TextPosition {
                start: 0,
                end: name.len() - 1,
            }],
            url: None,
            is_animated: false,
            width: Some(28),
            height: Some(28),
            metadata: EmoteMetadata {
                is_zero_width: false,
                modifier: false,
                emote_set_id: None,
                tier: None,
            },
        }
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = EmoteCache::new(1);
        let emote = create_test_emote("123", "test");

        cache.insert("123".to_string(), emote.clone());
        let retrieved = cache.get("123");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = EmoteCache::new(1);
        let retrieved = cache.get("nonexistent");

        assert!(retrieved.is_none());
        assert_eq!(cache.miss_count, 1);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = EmoteCache::new(1);
        let emote = create_test_emote("123", "test");

        cache.insert("123".to_string(), emote);
        cache.get("123");
        cache.get("123");
        cache.get("nonexistent");

        let stats = cache.stats();
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert_eq!(stats.hit_rate, 2.0 / 3.0);
    }
}
