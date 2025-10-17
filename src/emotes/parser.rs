use crate::connection::{Emote, EmoteSource, TextPosition};
use regex::Regex;
use std::collections::HashMap;

/// Parser de emotes que detecta y parsea emotes en texto de chat
pub struct EmoteParser {
    // Patrones regex para diferentes tipos de emotes
    twitch_emote_pattern: Regex,
    bttv_emote_pattern: Regex,
    ffz_emote_pattern: Regex,
    seven_tv_pattern: Regex,
    custom_patterns: HashMap<String, Regex>,
    known_emotes: HashMap<String, EmoteInfo>,
}

#[derive(Debug, Clone)]
pub struct EmoteInfo {
    pub id: String,
    pub name: String,
    pub source: EmoteSource,
    pub url: Option<String>,
    pub is_animated: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_zero_width: bool,
}

impl EmoteParser {
    pub fn new() -> Self {
        Self {
            twitch_emote_pattern: Regex::new(r"[A-Za-z0-9]+").unwrap(),
            bttv_emote_pattern: Regex::new(r"[A-Za-z0-9_]+").unwrap(),
            ffz_emote_pattern: Regex::new(r"[A-Za-z0-9]+").unwrap(),
            seven_tv_pattern: Regex::new(r"[A-Za-z0-9_]+").unwrap(),
            custom_patterns: HashMap::new(),
            known_emotes: HashMap::new(),
        }
    }

    /// Registra emotes conocidos para detección
    pub fn register_known_emotes(&mut self, emotes: Vec<EmoteInfo>) {
        for emote in emotes {
            self.known_emotes.insert(emote.name.clone(), emote);
        }
    }

    /// Registra un patrón personalizado para detección de emotes
    pub fn register_custom_pattern(&mut self, name: String, pattern: Regex) {
        self.custom_patterns.insert(name, pattern);
    }

    /// Parsea emotes desde datos crudos de Twitch
    pub fn parse_twitch_emotes(&self, message: &str, emote_data: &str) -> Vec<Emote> {
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

    /// Detecta emotes de terceros en un mensaje
    pub fn detect_third_party_emotes(&self, message: &str) -> Vec<Emote> {
        let mut emotes = Vec::new();
        let words: Vec<&str> = message.split_whitespace().collect();

        for (word_index, word) in words.iter().enumerate() {
            // Buscar en emotes conocidos
            if let Some(emote_info) = self.known_emotes.get(*word) {
                // Calcular posición del emote en el mensaje original
                let start_pos = if word_index == 0 {
                    0
                } else {
                    // Encontrar la posición sumando las longitudes de las palabras anteriores + espacios
                    words[..word_index]
                        .iter()
                        .map(|w| w.len() + 1) // +1 por el espacio
                        .sum::<usize>()
                };
                let end_pos = start_pos + word.len() - 1;

                emotes.push(Emote {
                    id: emote_info.id.clone(),
                    name: emote_info.name.clone(),
                    source: emote_info.source.clone(),
                    positions: vec![TextPosition {
                        start: start_pos,
                        end: end_pos,
                    }],
                    url: emote_info.url.clone(),
                    is_animated: emote_info.is_animated,
                    width: emote_info.width,
                    height: emote_info.height,
                    metadata: crate::connection::EmoteMetadata {
                        is_zero_width: emote_info.is_zero_width,
                        modifier: false,
                        emote_set_id: None,
                        tier: None,
                    },
                });
            }
        }

        emotes
    }

    /// Encuentra posiciones de un emote específico en el texto
    pub fn find_emote_positions(&self, text: &str, emote_name: &str) -> Vec<TextPosition> {
        let mut positions = Vec::new();
        let mut start = 0;

        while let Some(pos) = text[start..].find(emote_name) {
            let actual_start = start + pos;
            let actual_end = actual_start + emote_name.len() - 1;

            // Verificar que esté como palabra completa
            let prev_char = if actual_start > 0 {
                text.chars().nth(actual_start - 1)
            } else {
                None
            };

            let next_char = text.chars().nth(actual_end + 1);

            let is_word_boundary = match (prev_char, next_char) {
                (None, None) => true,
                (None, Some(c)) => !c.is_alphanumeric(),
                (Some(c), None) => !c.is_alphanumeric(),
                (Some(prev), Some(next)) => !prev.is_alphanumeric() && !next.is_alphanumeric(),
            };

            if is_word_boundary {
                positions.push(TextPosition {
                    start: actual_start,
                    end: actual_end,
                });
            }

            start = actual_end + 1;
        }

        positions
    }

    /// Parsea emotes usando todos los métodos disponibles
    pub async fn parse_all_emotes(
        &mut self,
        message: &str,
        platform_emote_data: &str,
        platform: &str,
    ) -> Vec<Emote> {
        let mut all_emotes = Vec::new();

        // Emotes de la plataforma (Twitch)
        if platform == "twitch" {
            let twitch_emotes = self.parse_twitch_emotes(message, platform_emote_data);
            all_emotes.extend(twitch_emotes);
        }

        // Emotes de terceros
        let third_party_emotes = self.detect_third_party_emotes(message);
        all_emotes.extend(third_party_emotes);

        // Eliminar duplicados y mantener el orden
        all_emotes = self.deduplicate_emotes(all_emotes);

        all_emotes
    }

    /// Elimina emotes duplicados manteniendo el orden
    fn deduplicate_emotes(&self, mut emotes: Vec<Emote>) -> Vec<Emote> {
        let mut seen_ids = std::collections::HashSet::new();
        let mut result = Vec::new();

        for emote in emotes {
            if !seen_ids.contains(&emote.id) {
                seen_ids.insert(emote.id.clone());
                result.push(emote);
            }
        }

        result
    }

    /// Extrae texto plano del mensaje reemplazando emotes con placeholders
    pub fn extract_plain_text(&self, message: &str, emotes: &[Emote]) -> String {
        let mut result = message.to_string();

        // Ordenar emotes por posición de inicio en orden descendente
        let mut sorted_emotes = emotes.to_vec();
        sorted_emotes.sort_by(|a, b| {
            let a_start = a.positions.first().map(|p| p.start).unwrap_or(0);
            let b_start = b.positions.first().map(|p| p.start).unwrap_or(0);
            b_start.cmp(&a_start)
        });

        for emote in sorted_emotes {
            for position in &emote.positions {
                if position.start < result.len() && position.end < result.len() {
                    result
                        .replace_range(position.start..=position.end, &format!(":{}", emote.name));
                }
            }
        }

        result
    }

    /// Valida si un texto es un emote válido
    pub fn is_valid_emote(&self, text: &str) -> bool {
        // Verificar si está en la lista de emotes conocidos
        if self.known_emotes.contains_key(text) {
            return true;
        }

        // Verificar patrones regex
        self.twitch_emote_pattern.is_match(text)
            || self.bttv_emote_pattern.is_match(text)
            || self.ffz_emote_pattern.is_match(text)
            || self.seven_tv_pattern.is_match(text)
    }

    /// Obtiene información de un emote conocido
    pub fn get_emote_info(&self, name: &str) -> Option<&EmoteInfo> {
        self.known_emotes.get(name)
    }

    /// Limpia los emotes conocidos
    pub fn clear_known_emotes(&mut self) {
        self.known_emotes.clear();
    }

    /// Obtiene estadísticas del parser
    pub fn get_stats(&self) -> ParserStats {
        ParserStats {
            known_emotes_count: self.known_emotes.len(),
            custom_patterns_count: self.custom_patterns.len(),
            sources: self.get_emote_sources_distribution(),
        }
    }

    /// Obtiene distribución de fuentes de emotes
    fn get_emote_sources_distribution(&self) -> HashMap<EmoteSource, usize> {
        let mut distribution = HashMap::new();

        for emote_info in self.known_emotes.values() {
            *distribution.entry(emote_info.source.clone()).or_insert(0) += 1;
        }

        distribution
    }
}

/// Estadísticas del parser
#[derive(Debug, Clone)]
pub struct ParserStats {
    pub known_emotes_count: usize,
    pub custom_patterns_count: usize,
    pub sources: HashMap<EmoteSource, usize>,
}

impl Default for EmoteParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::EmoteMetadata;

    #[test]
    fn test_parse_twitch_emotes() {
        let parser = EmoteParser::new();
        let message = "Hello Kappa world PogChamp";
        let emote_data = "25:6-10/305954156:12-19";

        let emotes = parser.parse_twitch_emotes(message, emote_data);

        assert_eq!(emotes.len(), 2);
        assert_eq!(emotes[0].name, "Kappa");
        assert_eq!(emotes[1].name, "PogChamp");
    }

    #[test]
    fn test_detect_third_party_emotes() {
        let mut parser = EmoteParser::new();

        // Registrar emotes conocidos
        parser.register_known_emotes(vec![EmoteInfo {
            id: "bttv123".to_string(),
            name: "FeelsBadMan".to_string(),
            source: EmoteSource::BTTV,
            url: None,
            is_animated: false,
            width: None,
            height: None,
            is_zero_width: false,
        }]);

        let message = "This is FeelsBadMan moment";
        let emotes = parser.detect_third_party_emotes(message);

        assert_eq!(emotes.len(), 1);
        assert_eq!(emotes[0].name, "FeelsBadMan");
        assert_eq!(emotes[0].source, EmoteSource::BTTV);
    }

    #[test]
    fn test_find_emote_positions() {
        let parser = EmoteParser::new();
        let text = "Hello Kappa world Kappa";
        let positions = parser.find_emote_positions(text, "Kappa");

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].start, 6);
        assert_eq!(positions[0].end, 10);
        assert_eq!(positions[1].start, 18);
        assert_eq!(positions[1].end, 22);
    }

    #[test]
    fn test_extract_plain_text() {
        let parser = EmoteParser::new();
        let message = "Hello Kappa world";
        let emotes = vec![Emote {
            id: "25".to_string(),
            name: "Kappa".to_string(),
            source: EmoteSource::Twitch,
            positions: vec![TextPosition { start: 6, end: 10 }],
            url: None,
            is_animated: false,
            width: None,
            height: None,
            metadata: EmoteMetadata::default(),
        }];

        let plain = parser.extract_plain_text(message, &emotes);
        assert_eq!(plain, "Hello :Kappa world");
    }
}
