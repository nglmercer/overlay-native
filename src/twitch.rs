use async_trait::async_trait;
use std::time::Instant;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::{PrivmsgMessage, ServerMessage};
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};
use tokio::sync::mpsc;

use crate::connection::{ChatEmote, ChatMessage, StreamingPlatform};

#[derive(Debug)]
pub enum TwitchError {
    ConnectionError(String),
    JoinError(String),
}

impl std::fmt::Display for TwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwitchError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            TwitchError::JoinError(msg) => write!(f, "Join error: {}", msg),
        }
    }
}

impl std::error::Error for TwitchError {}

pub struct TwitchPlatform {
    client: Option<TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>>,
    message_receiver: Option<mpsc::UnboundedReceiver<ServerMessage>>,
    connected: bool,
}

impl TwitchPlatform {
    pub fn new() -> Self {
        Self {
            client: None,
            message_receiver: None,
            connected: false,
        }
    }
    
    fn convert_emotes(emotes: &[twitch_irc::message::Emote]) -> Vec<ChatEmote> {
        emotes
            .iter()
            .map(|emote| ChatEmote {
                id: emote.id.clone(),
                name: emote.code.clone(),
                positions: vec![(emote.char_range.start, emote.char_range.end)],
            })
            .collect()
    }
    
    fn convert_message(msg: PrivmsgMessage) -> ChatMessage {
        ChatMessage {
            username: msg.sender.name,
            content: msg.message_text,
            emotes: Self::convert_emotes(&msg.emotes),
            timestamp: Instant::now(),
        }
    }
}

#[async_trait]
impl StreamingPlatform for TwitchPlatform {
    type Error = TwitchError;
    
    async fn connect(&mut self) -> Result<(), Self::Error> {
        let config: ClientConfig<StaticLoginCredentials> = ClientConfig::default();
        let (incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
        
        self.client = Some(client);
        self.message_receiver = Some(incoming_messages);
        self.connected = true;
        
        Ok(())
    }
    
    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> {
        if let Some(client) = &self.client {
            client
                .join(channel.to_owned())
                .map_err(|e| TwitchError::JoinError(e.to_string()))?;
            Ok(())
        } else {
            Err(TwitchError::ConnectionError(
                "Not connected to Twitch".to_string(),
            ))
        }
    }
    
    async fn next_message(&mut self) -> Option<ChatMessage> {
        if let Some(receiver) = &mut self.message_receiver {
            while let Some(message) = receiver.recv().await {
                match message {
                    ServerMessage::Privmsg(privmsg) => {
                        return Some(Self::convert_message(privmsg));
                    }
                    ServerMessage::Ping(_) | ServerMessage::Pong(_) => {
                        // Ignorar mensajes de ping/pong
                        continue;
                    }
                    _ => {
                        // Otros mensajes pueden ser loggeados si es necesario
                        continue;
                    }
                }
            }
        }
        None
    }
    
    async fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.connected = false;
        self.client = None;
        self.message_receiver = None;
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Default for TwitchPlatform {
    fn default() -> Self {
        Self::new()
    }
}