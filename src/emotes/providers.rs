use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::{EmoteData, EmoteError, EmoteProvider};
use crate::connection::{Emote, EmoteSource, TextPosition};

/// BTTV Emote structure
#[derive(Deserialize)]
struct BTTVEmote {
    id: String,
    code: String,
    #[serde(rename = "imageType")]
    image_type: String,
    animated: bool,
    #[serde(rename = "userId")]
    user_id: String,
    modifier: bool,
    width: Option<u32>,
    height: Option<u32>,
}

/// BTTV Global Response is a direct array of emotes
type BTTVGlobalResponse = Vec<BTTVEmote>;

/// Cliente HTTP para APIs de emotes
#[derive(Clone)]
pub struct EmoteApiClient {
    client: Client,
    timeout: Duration,
}

impl EmoteApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Overlay-Native/1.0")
                .build()
                .unwrap_or_default(),
            timeout: Duration::from_secs(10),
        }
    }

    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, EmoteError> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 500;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = BASE_DELAY_MS * (2_u64.pow(attempt - 1));
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            match self.try_get_json(url).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        eprintln!(
                            "      ⚠️  Attempt {}/{} failed for {}: {}. Retrying...",
                            attempt + 1,
                            MAX_RETRIES,
                            url,
                            e
                        );
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| EmoteError::NetworkError("Unknown error".to_string())))
    }

    async fn try_get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, EmoteError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| EmoteError::NetworkError(format!("Failed to fetch {}: {}", url, e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(EmoteError::NetworkError(format!(
                "HTTP {} from {}: {}",
                status.as_u16(),
                url,
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read response body".to_string())
            )));
        }

        response.json().await.map_err(|e| {
            EmoteError::NetworkError(format!("Failed to parse JSON from {}: {}", url, e))
        })
    }
}

impl Default for EmoteApiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Proveedor de emotes de Twitch
pub struct TwitchEmoteProvider {
    api_client: EmoteApiClient,
}

impl TwitchEmoteProvider {
    pub fn new() -> Self {
        Self {
            api_client: EmoteApiClient::new(),
        }
    }

    fn parse_twitch_emote_data(&self, message: &str, emote_data: &str) -> Vec<Emote> {
        let mut emotes = Vec::new();

        if emote_data.is_empty() {
            return emotes;
        }

        // Formato: "emote_id:start-end,start-end/emote_id2:..."
        for emote_part in emote_data.split('/') {
            let parts: Vec<&str> = emote_part.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let emote_id = parts[0];
            let positions = parts[1];

            for position in positions.split(',') {
                let pos_parts: Vec<&str> = position.split('-').collect();
                if pos_parts.len() != 2 {
                    continue;
                }

                if let (Ok(start), Ok(end)) =
                    (pos_parts[0].parse::<usize>(), pos_parts[1].parse::<usize>())
                {
                    if start < message.len() && end < message.len() {
                        let emote_name = message[start..=end].to_string();

                        let source = if emote_id.starts_with("emotesv2_") {
                            EmoteSource::TwitchGlobal
                        } else if emote_id.chars().all(|c| c.is_ascii_digit()) {
                            EmoteSource::TwitchSubscriber
                        } else {
                            EmoteSource::Twitch
                        };

                        emotes.push(Emote {
                            id: emote_id.to_string(),
                            name: emote_name,
                            source,
                            positions: vec![TextPosition { start, end }],
                            url: Some(format!(
                                "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/1.0",
                                emote_id
                            )),
                            is_animated: false,
                            width: Some(28),
                            height: Some(28),
                            metadata: crate::connection::EmoteMetadata {
                                is_zero_width: false,
                                modifier: false,
                                emote_set_id: Some(emote_id.to_string()),
                                tier: None,
                            },
                        });
                    }
                }
            }
        }

        emotes
    }
}

#[async_trait]
impl EmoteProvider for TwitchEmoteProvider {
    async fn parse_emotes(
        &self,
        message: &str,
        emote_data: &str,
    ) -> Result<Vec<Emote>, EmoteError> {
        Ok(self.parse_twitch_emote_data(message, emote_data))
    }

    async fn get_channel_emotes(
        &self,
        _platform: &str,
        channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError> {
        // Esto requeriría la API de Twitch que necesita autenticación
        // Por ahora implementamos un simulación
        let url = format!(
            "https://api.twitch.tv/helix/chat/emotes?broadcaster_id={}",
            channel
        );

        #[derive(Deserialize)]
        struct TwitchEmoteResponse {
            data: Vec<TwitchEmote>,
        }

        #[derive(Deserialize)]
        struct TwitchEmote {
            id: String,
            name: String,
            images: TwitchEmoteImages,
            emote_type: String,
            tier: Option<String>,
        }

        #[derive(Deserialize)]
        struct TwitchEmoteImages {
            #[serde(rename = "url_1x")]
            url_1x: String,
            #[serde(rename = "url_2x")]
            url_2x: String,
            #[serde(rename = "url_4x")]
            url_4x: String,
        }

        // Simulación por ahora
        Ok(vec![])
    }

    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        // Emotes globales de Twitch
        Ok(vec![])
    }

    fn provider_name(&self) -> &str {
        "twitch"
    }
}

impl Default for TwitchEmoteProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Proveedor de emotes BetterTTV
pub struct BTTVEmoteProvider {
    api_client: EmoteApiClient,
}

impl BTTVEmoteProvider {
    pub fn new() -> Self {
        Self {
            api_client: EmoteApiClient::new(),
        }
    }

    async fn get_bttv_channel_emotes(&self, channel: &str) -> Result<Vec<EmoteData>, EmoteError> {
        let url = format!(
            "https://api.betterttv.net/3/cached/users/twitch/{}",
            channel
        );

        #[derive(Deserialize)]
        struct BTTVUserResponse {
            channel_emotes: Vec<BTTVEmote>,
            shared_emotes: Vec<BTTVEmote>,
        }

        let response: BTTVUserResponse = self.api_client.get_json(&url).await?;
        let mut emotes = Vec::new();

        for emote in response.channel_emotes {
            let emote_id = emote.id.clone();
            emotes.push(EmoteData {
                id: emote_id.clone(),
                name: emote.code,
                url: Some(format!("https://cdn.betterttv.net/emote/{}/3x", emote_id)),
                is_animated: emote.animated,
                width: emote.width,
                height: emote.height,
                is_zero_width: false,
                modifier: false,
                emote_set_id: None,
            });
        }

        for emote in response.shared_emotes {
            let emote_id = emote.id.clone();
            emotes.push(EmoteData {
                id: emote_id.clone(),
                name: emote.code,
                url: Some(format!("https://cdn.betterttv.net/emote/{}/3x", emote_id)),
                is_animated: emote.animated,
                width: emote.width,
                height: emote.height,
                is_zero_width: false,
                modifier: false,
                emote_set_id: None,
            });
        }

        Ok(emotes)
    }

    async fn get_bttv_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        let url = "https://api.betterttv.net/3/cached/emotes/global";

        let response = self.api_client.get_json::<BTTVGlobalResponse>(url).await?;
        let mut emotes = Vec::new();

        for emote in response {
            let emote_id = emote.id.clone();
            emotes.push(EmoteData {
                id: emote_id.clone(),
                name: emote.code,
                url: Some(format!("https://cdn.betterttv.net/emote/{}/3x", emote_id)),
                is_animated: emote.animated,
                width: emote.width,
                height: emote.height,
                is_zero_width: false,
                modifier: emote.modifier,
                emote_set_id: None,
            });
        }

        Ok(emotes)
    }
}

#[async_trait]
impl EmoteProvider for BTTVEmoteProvider {
    async fn parse_emotes(
        &self,
        message: &str,
        _emote_data: &str,
    ) -> Result<Vec<Emote>, EmoteError> {
        // BTTV no proporciona datos de emotes en el mensaje, se debe buscar por nombre
        // Esto se maneja en el sistema principal
        Ok(vec![])
    }

    async fn get_channel_emotes(
        &self,
        platform: &str,
        channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError> {
        if platform != "twitch" {
            return Ok(vec![]);
        }

        self.get_bttv_channel_emotes(channel).await
    }

    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        self.get_bttv_global_emotes().await
    }

    fn provider_name(&self) -> &str {
        "bttv"
    }
}

impl Default for BTTVEmoteProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Proveedor de emotes FrankerFaceZ
pub struct FFZEmoteProvider {
    api_client: EmoteApiClient,
}

impl FFZEmoteProvider {
    pub fn new() -> Self {
        Self {
            api_client: EmoteApiClient::new(),
        }
    }

    async fn get_ffz_channel_emotes(&self, channel: &str) -> Result<Vec<EmoteData>, EmoteError> {
        let url = format!("https://api.frankerfacez.com/v1/room/{}", channel);

        #[derive(Deserialize)]
        struct FFZRoomResponse {
            room: FFZRoom,
            sets: HashMap<String, FFZSet>,
        }

        #[derive(Deserialize)]
        struct FFZRoom {
            id: u32,
            set: u32,
        }

        #[derive(Deserialize)]
        struct FFZSet {
            emoticons: Vec<FFZEmote>,
        }

        #[derive(Deserialize)]
        struct FFZEmote {
            id: u32,
            name: String,
            urls: HashMap<String, String>,
            animated: Option<bool>,
        }

        let response: FFZRoomResponse = self.api_client.get_json(&url).await?;
        let mut emotes = Vec::new();

        if let Some(set) = response.sets.get(&response.room.set.to_string()) {
            for emote in &set.emoticons {
                let url = emote
                    .urls
                    .get("4")
                    .or_else(|| emote.urls.get("2"))
                    .or_else(|| emote.urls.get("1"))
                    .cloned();

                emotes.push(EmoteData {
                    id: emote.id.to_string(),
                    name: emote.name.clone(),
                    url,
                    is_animated: emote.animated.unwrap_or(false),
                    width: None,
                    height: None,
                    is_zero_width: false,
                    modifier: false,
                    emote_set_id: Some(response.room.set.to_string()),
                });
            }
        }

        Ok(emotes)
    }

    async fn get_ffz_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        let url = "https://api.frankerfacez.com/v1/set/global";

        #[derive(Deserialize)]
        struct FFZGlobalResponse {
            sets: HashMap<String, FFZSet>,
        }

        #[derive(Deserialize)]
        struct FFZSet {
            emoticons: Vec<FFZEmote>,
        }

        #[derive(Deserialize)]
        struct FFZEmote {
            id: u32,
            name: String,
            urls: HashMap<String, String>,
            animated: Option<bool>,
        }

        let response: FFZGlobalResponse = self.api_client.get_json(&url).await?;
        let mut emotes = Vec::new();

        for set in response.sets.values() {
            for emote in &set.emoticons {
                let url = emote
                    .urls
                    .get("4")
                    .or_else(|| emote.urls.get("2"))
                    .or_else(|| emote.urls.get("1"))
                    .cloned();

                emotes.push(EmoteData {
                    id: emote.id.to_string(),
                    name: emote.name.clone(),
                    url,
                    is_animated: emote.animated.unwrap_or(false),
                    width: None,
                    height: None,
                    is_zero_width: false,
                    modifier: false,
                    emote_set_id: None,
                });
            }
        }

        Ok(emotes)
    }
}

#[async_trait]
impl EmoteProvider for FFZEmoteProvider {
    async fn parse_emotes(
        &self,
        _message: &str,
        _emote_data: &str,
    ) -> Result<Vec<Emote>, EmoteError> {
        // FFZ no proporciona datos de emotes en el mensaje
        Ok(vec![])
    }

    async fn get_channel_emotes(
        &self,
        platform: &str,
        channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError> {
        if platform != "twitch" {
            return Ok(vec![]);
        }

        self.get_ffz_channel_emotes(channel).await
    }

    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        self.get_ffz_global_emotes().await
    }

    fn provider_name(&self) -> &str {
        "ffz"
    }
}

impl Default for FFZEmoteProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Proveedor de emotes 7TV
pub struct SevenTVEmoteProvider {
    api_client: EmoteApiClient,
}

impl SevenTVEmoteProvider {
    pub fn new() -> Self {
        Self {
            api_client: EmoteApiClient::new(),
        }
    }

    async fn get_7tv_channel_emotes(&self, channel: &str) -> Result<Vec<EmoteData>, EmoteError> {
        let url = format!("https://7tv.io/v3/users/twitch/{}", channel);

        #[derive(Deserialize)]
        struct SevenTVUserResponse {
            emote_set: Option<SevenTVEmoteSet>,
        }

        #[derive(Deserialize)]
        struct SevenTVEmoteSet {
            emotes: Vec<SevenTVEmote>,
        }

        #[derive(Deserialize)]
        struct SevenTVEmote {
            id: String,
            name: String,
            data: SevenTVEmoteData,
        }

        #[derive(Deserialize)]
        struct SevenTVEmoteData {
            name: String,
            flags: u32,
            animated: bool,
        }

        let response: SevenTVUserResponse = self.api_client.get_json(&url).await?;
        let mut emotes = Vec::new();

        if let Some(set) = response.emote_set {
            for emote in set.emotes {
                let emote_id = emote.id.clone();
                emotes.push(EmoteData {
                    id: emote_id.clone(),
                    name: emote.name,
                    url: Some(format!("https://cdn.7tv.app/emote/{}/4x", emote_id)),
                    is_animated: emote.data.animated,
                    width: None,
                    height: None,
                    is_zero_width: (emote.data.flags & 1) != 0, // Zero width flag
                    modifier: (emote.data.flags & 2) != 0,      // Modifier flag
                    emote_set_id: None,
                });
            }
        }

        Ok(emotes)
    }

    async fn get_7tv_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        let url = "https://7tv.io/v3/emote-sets/global";

        #[derive(Deserialize)]
        struct SevenTVGlobalResponse {
            emotes: Vec<SevenTVEmote>,
        }

        #[derive(Deserialize)]
        struct SevenTVEmote {
            id: String,
            name: String,
            data: SevenTVEmoteData,
        }

        #[derive(Deserialize)]
        struct SevenTVEmoteData {
            name: String,
            flags: u32,
            animated: bool,
        }

        let response: SevenTVGlobalResponse = self.api_client.get_json(&url).await?;
        let mut emotes = Vec::new();

        for emote in response.emotes {
            let emote_id = emote.id.clone();
            emotes.push(EmoteData {
                id: emote_id.clone(),
                name: emote.name,
                url: Some(format!("https://cdn.7tv.app/emote/{}/4x", emote_id)),
                is_animated: emote.data.animated,
                width: None,
                height: None,
                is_zero_width: (emote.data.flags & 1) != 0,
                modifier: (emote.data.flags & 2) != 0,
                emote_set_id: None,
            });
        }

        Ok(emotes)
    }
}

#[async_trait]
impl EmoteProvider for SevenTVEmoteProvider {
    async fn parse_emotes(
        &self,
        _message: &str,
        _emote_data: &str,
    ) -> Result<Vec<Emote>, EmoteError> {
        // 7TV no proporciona datos de emotes en el mensaje
        Ok(vec![])
    }

    async fn get_channel_emotes(
        &self,
        platform: &str,
        channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError> {
        if platform != "twitch" {
            return Ok(vec![]);
        }

        self.get_7tv_channel_emotes(channel).await
    }

    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        self.get_7tv_global_emotes().await
    }

    fn provider_name(&self) -> &str {
        "7tv"
    }
}

impl Default for SevenTVEmoteProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bttv_global_emotes() {
        let provider = BTTVEmoteProvider::new();
        match provider.get_global_emotes().await {
            Ok(emotes) => {
                println!("✅ BTTV: Loaded {} global emotes", emotes.len());
                assert!(!emotes.is_empty(), "BTTV should have global emotes");

                // Print first few emotes for debugging
                for emote in emotes.iter().take(3) {
                    println!("   - {} ({})", emote.name, emote.id);
                }
            }
            Err(e) => {
                panic!("❌ BTTV failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_ffz_global_emotes() {
        let provider = FFZEmoteProvider::new();
        match provider.get_global_emotes().await {
            Ok(emotes) => {
                println!("✅ FFZ: Loaded {} global emotes", emotes.len());
                assert!(!emotes.is_empty(), "FFZ should have global emotes");

                // Print first few emotes for debugging
                for emote in emotes.iter().take(3) {
                    println!("   - {} ({})", emote.name, emote.id);
                }
            }
            Err(e) => {
                panic!("❌ FFZ failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_7tv_global_emotes() {
        let provider = SevenTVEmoteProvider::new();
        match provider.get_global_emotes().await {
            Ok(emotes) => {
                println!("✅ 7TV: Loaded {} global emotes", emotes.len());
                assert!(!emotes.is_empty(), "7TV should have global emotes");

                // Print first few emotes for debugging
                for emote in emotes.iter().take(3) {
                    println!("   - {} ({})", emote.name, emote.id);
                }
            }
            Err(e) => {
                panic!("❌ 7TV failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_twitch_provider() {
        let provider = TwitchEmoteProvider::new();
        // Twitch doesn't have a public global emotes endpoint without auth
        // Just test that the provider exists
        assert_eq!(provider.provider_name(), "twitch");
        println!("✅ Twitch provider initialized");
    }

    #[tokio::test]
    async fn test_api_client() {
        let client = EmoteApiClient::new();

        // Test with a simple endpoint
        let url = "https://api.betterttv.net/3/cached/emotes/global";
        match client.get_json::<BTTVGlobalResponse>(url).await {
            Ok(response) => {
                println!("✅ API Client works: {} emotes fetched", response.len());
            }
            Err(e) => {
                panic!("❌ API Client failed: {}", e);
            }
        }
    }

    /// Test all providers to identify which one fails
    #[tokio::test]
    async fn test_all_providers_individually() {
        println!("\n🔍 Testing all providers individually...\n");

        let providers: Vec<(&str, Box<dyn EmoteProvider>)> = vec![
            ("BTTV", Box::new(BTTVEmoteProvider::new())),
            ("FFZ", Box::new(FFZEmoteProvider::new())),
            ("7TV", Box::new(SevenTVEmoteProvider::new())),
        ];

        let mut results = Vec::new();

        for (name, provider) in providers {
            print!("Testing {}... ", name);
            match provider.get_global_emotes().await {
                Ok(emotes) => {
                    println!("✅ {} emotes", emotes.len());
                    results.push((name, true, emotes.len()));
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                    results.push((name, false, 0));
                }
            }
        }

        println!("\n📊 Results Summary:");
        for (name, success, count) in results {
            if success {
                println!("   ✅ {}: {} emotes", name, count);
            } else {
                println!("   ❌ {}: FAILED", name);
            }
        }
    }
}
