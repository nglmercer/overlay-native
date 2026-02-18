/// Schema de mensajes entrantes para la capa de transporte (IPC/WebSocket)
///
/// Este módulo define el formato de mensajes que los clientes deben enviar
/// al overlay. La plataforma es agnóstica: cualquier fuente puede enviar
/// mensajes mientras cumplan este esquema.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error de validación de mensajes
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Campo requerido faltante: {0}")]
    MissingField(String),

    #[error("Valor inválido para '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("JSON inválido: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Tipo de mensaje desconocido: {0}")]
    UnknownMessageType(String),
}

/// Mensaje entrante desde cualquier transporte (WebSocket, IPC)
/// Usa el campo `type` como discriminante (serde tagged enum)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum IncomingMessage {
    /// Mensaje de chat estándar (texto, emotes, badges)
    ChatMessage(ChatMessagePayload),

    /// Regalo/Gift (subscripción, bits, etc.)
    Gift(GiftPayload),

    /// Evento de emote único (sticker, solo emote)
    Emote(EmoteEventPayload),

    /// Ping para mantener la conexión viva
    Ping,

    /// Solicitud de estado del overlay
    Status,
}

/// Payload de un mensaje de chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    /// ID único del mensaje (opcional, se genera si no se provee)
    pub id: Option<String>,

    /// Nombre de usuario (requerido)
    pub username: String,

    /// Nombre visible del usuario (opcional, usa `username` si no se provee)
    pub display_name: Option<String>,

    /// Contenido del mensaje (requerido)
    pub content: String,

    /// Color del usuario en formato hex (ej: "#FF0000")
    pub user_color: Option<String>,

    /// Lista de emotes en el mensaje
    #[serde(default)]
    pub emotes: Vec<EmotePayload>,

    /// Lista de badges del usuario
    #[serde(default)]
    pub badges: Vec<BadgePayload>,

    /// Plataforma de origen (informativo, no afecta renderizado)
    pub platform: Option<String>,

    /// Canal de origen (informativo, no afecta renderizado)
    pub channel: Option<String>,

    /// Metadatos adicionales arbitrarios
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Payload de un regalo/gift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPayload {
    /// Quien envía el regalo
    pub from_user: String,

    /// Quien lo recibe (None = aleatorio/comunidad)
    pub to_user: Option<String>,

    /// Tipo de regalo (subscription, bits, etc.)
    pub gift_type: GiftType,

    /// Cantidad (meses de suscripción, bits, etc.)
    pub amount: Option<u32>,

    /// Nombre del tier/plan (Tier 1, Tier 2, etc.)
    pub tier: Option<String>,

    /// Mensaje personalizado
    pub message: Option<String>,

    /// Plataforma de origen (informativo)
    pub platform: Option<String>,
}

/// Tipo de regalo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiftType {
    Subscription,
    GiftSubscription,
    Bits,
    Cheer,
    Donation,
    Other(String),
}

/// Payload de un evento de emote único (sticker, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmoteEventPayload {
    /// ID del emote
    pub id: String,

    /// Nombre del emote
    pub name: String,

    /// URL de la imagen del emote
    pub url: String,

    /// Si el emote es animado (GIF)
    #[serde(default)]
    pub is_animated: bool,

    /// Ancho en píxeles
    pub width: Option<u32>,

    /// Alto en píxeles
    pub height: Option<u32>,

    /// Usuario que lo envió (opcional)
    pub sender: Option<String>,
}

/// Definición de un emote dentro de un mensaje
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotePayload {
    /// ID del emote en su plataforma/servicio
    pub id: String,

    /// Nombre/código del emote (ej: "Kappa", "monkaS")
    pub name: String,

    /// URL directa a la imagen del emote
    pub url: Option<String>,

    /// Si el emote es animado (GIF/WebP animado)
    #[serde(default)]
    pub is_animated: bool,

    /// Posiciones en el texto donde aparece el emote
    #[serde(default)]
    pub positions: Vec<EmotePosition>,

    /// Ancho del emote en píxeles
    pub width: Option<u32>,

    /// Alto del emote en píxeles
    pub height: Option<u32>,
}

/// Posición de un emote en el texto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotePosition {
    pub start: usize,
    pub end: usize,
}

/// Definición de un badge de usuario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgePayload {
    /// ID del badge
    pub id: String,

    /// Nombre legible del badge
    pub name: String,

    /// URL de la imagen del badge
    pub url: Option<String>,

    /// Descripción/título del badge
    pub title: Option<String>,
}

/// Respuesta del overlay al cliente
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum OutgoingMessage {
    /// Confirmación de mensaje recibido
    Ack { id: String },

    /// Respuesta al ping
    Pong,

    /// Estado actual del overlay
    Status(OverlayStatus),

    /// Error de procesamiento
    Error { code: String, message: String },
}

/// Estado del overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayStatus {
    /// Número de ventanas activas
    pub active_windows: usize,

    /// Número de clientes conectados
    pub connected_clients: usize,

    /// Versión del overlay
    pub version: String,
}

impl IncomingMessage {
    /// Parsea un mensaje desde JSON y lo valida
    pub fn parse_and_validate(raw: &str) -> Result<Self, SchemaError> {
        let msg: IncomingMessage = serde_json::from_str(raw)?;
        msg.validate()?;
        Ok(msg)
    }

    /// Valida las reglas de negocio del mensaje
    pub fn validate(&self) -> Result<(), SchemaError> {
        match self {
            IncomingMessage::ChatMessage(payload) => payload.validate(),
            IncomingMessage::Gift(payload) => payload.validate(),
            IncomingMessage::Emote(payload) => payload.validate(),
            IncomingMessage::Ping | IncomingMessage::Status => Ok(()),
        }
    }
}

impl ChatMessagePayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        // username es requerido y no puede estar vacío
        if self.username.trim().is_empty() {
            return Err(SchemaError::MissingField("username".to_string()));
        }

        // content es requerido y no puede estar vacío
        if self.content.trim().is_empty() {
            return Err(SchemaError::MissingField("content".to_string()));
        }

        // Longitud máxima de username
        if self.username.len() > 256 {
            return Err(SchemaError::InvalidValue {
                field: "username".to_string(),
                reason: "demasiado largo (máximo 256 caracteres)".to_string(),
            });
        }

        // Longitud máxima de content
        if self.content.len() > 4096 {
            return Err(SchemaError::InvalidValue {
                field: "content".to_string(),
                reason: "demasiado largo (máximo 4096 caracteres)".to_string(),
            });
        }

        // Validar color si se provee
        if let Some(color) = &self.user_color {
            if !is_valid_hex_color(color) {
                return Err(SchemaError::InvalidValue {
                    field: "user_color".to_string(),
                    reason: format!("'{}' no es un color hex válido (ej: #FF0000)", color),
                });
            }
        }

        // Validar posiciones de emotes
        for (i, emote) in self.emotes.iter().enumerate() {
            for pos in &emote.positions {
                if pos.start > pos.end {
                    return Err(SchemaError::InvalidValue {
                        field: format!("emotes[{}].positions", i),
                        reason: format!("start ({}) no puede ser mayor que end ({})", pos.start, pos.end),
                    });
                }
                if pos.end > self.content.len() {
                    return Err(SchemaError::InvalidValue {
                        field: format!("emotes[{}].positions", i),
                        reason: format!(
                            "end ({}) excede la longitud del contenido ({})",
                            pos.end,
                            self.content.len()
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Genera un ID si no se provee uno
    pub fn get_or_generate_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("msg_{}_{}", ts, rand::random::<u32>())
        })
    }
}

impl GiftPayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.from_user.trim().is_empty() {
            return Err(SchemaError::MissingField("from_user".to_string()));
        }
        Ok(())
    }
}

impl EmoteEventPayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.id.trim().is_empty() {
            return Err(SchemaError::MissingField("id".to_string()));
        }
        if self.name.trim().is_empty() {
            return Err(SchemaError::MissingField("name".to_string()));
        }
        if self.url.trim().is_empty() {
            return Err(SchemaError::MissingField("url".to_string()));
        }
        Ok(())
    }
}

/// Valida si un string es un color hexadecimal válido (#RGB, #RRGGBB, #RRGGBBAA)
fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    let valid_length = matches!(hex.len(), 3 | 4 | 6 | 8);
    let valid_chars = hex.chars().all(|c| c.is_ascii_hexdigit());
    valid_length && valid_chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chat_message() {
        let json = r#"{
            "type": "chat_message",
            "data": {
                "username": "testuser",
                "content": "Hola mundo!",
                "user_color": "#FF6600"
            }
        }"#;

        let msg = IncomingMessage::parse_and_validate(json).unwrap();
        assert!(matches!(msg, IncomingMessage::ChatMessage(_)));
    }

    #[test]
    fn test_parse_chat_message_with_emotes() {
        let json = r#"{
            "type": "chat_message",
            "data": {
                "username": "user123",
                "content": "Kappa monkaS",
                "emotes": [
                    {
                        "id": "25",
                        "name": "Kappa",
                        "url": "https://cdn.example.com/emote/25",
                        "positions": [{"start": 0, "end": 5}]
                    }
                ]
            }
        }"#;

        let msg = IncomingMessage::parse_and_validate(json).unwrap();
        if let IncomingMessage::ChatMessage(payload) = msg {
            assert_eq!(payload.username, "user123");
            assert_eq!(payload.emotes.len(), 1);
            assert_eq!(payload.emotes[0].name, "Kappa");
        } else {
            panic!("Expected ChatMessage");
        }
    }

    #[test]
    fn test_empty_username_fails() {
        let json = r#"{
            "type": "chat_message",
            "data": {
                "username": "",
                "content": "Hello"
            }
        }"#;

        assert!(IncomingMessage::parse_and_validate(json).is_err());
    }

    #[test]
    fn test_empty_content_fails() {
        let json = r#"{
            "type": "chat_message",
            "data": {
                "username": "user",
                "content": ""
            }
        }"#;

        assert!(IncomingMessage::parse_and_validate(json).is_err());
    }

    #[test]
    fn test_invalid_hex_color_fails() {
        let json = r#"{
            "type": "chat_message",
            "data": {
                "username": "user",
                "content": "Hello",
                "user_color": "red"
            }
        }"#;

        assert!(IncomingMessage::parse_and_validate(json).is_err());
    }

    #[test]
    fn test_ping_message() {
        let json = r#"{"type": "ping"}"#;
        let msg = IncomingMessage::parse_and_validate(json).unwrap();
        assert!(matches!(msg, IncomingMessage::Ping));
    }

    #[test]
    fn test_gift_message() {
        let json = r#"{
            "type": "gift",
            "data": {
                "from_user": "donator",
                "gift_type": "subscription",
                "amount": 1
            }
        }"#;

        let msg = IncomingMessage::parse_and_validate(json).unwrap();
        assert!(matches!(msg, IncomingMessage::Gift(_)));
    }

    #[test]
    fn test_hex_color_validation() {
        assert!(is_valid_hex_color("#FF0000"));
        assert!(is_valid_hex_color("#f00"));
        assert!(is_valid_hex_color("#FF000080"));
        assert!(!is_valid_hex_color("FF0000"));
        assert!(!is_valid_hex_color("#ZZZZZZ"));
        assert!(!is_valid_hex_color("#12345"));
    }
}
