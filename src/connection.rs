use async_trait::async_trait;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub username: String,
    pub content: String,
    pub emotes: Vec<ChatEmote>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct ChatEmote {
    pub id: String,
    pub name: String,
    pub positions: Vec<(usize, usize)>,
}

#[async_trait]
pub trait StreamingPlatform {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Conecta a la plataforma de streaming
    async fn connect(&mut self) -> Result<(), Self::Error>;
    
    /// Se une a un canal específico
    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error>;
    
    /// Obtiene el siguiente mensaje del chat
    async fn next_message(&mut self) -> Option<ChatMessage>;
    
    /// Desconecta de la plataforma
    async fn disconnect(&mut self) -> Result<(), Self::Error>;
    
    /// Verifica si la conexión está activa
    fn is_connected(&self) -> bool;
}

pub struct PlatformManager {
    message_sender: mpsc::UnboundedSender<ChatMessage>,
    message_receiver: mpsc::UnboundedReceiver<ChatMessage>,
}

impl PlatformManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            message_sender: sender,
            message_receiver: receiver,
        }
    }
    
    pub fn get_sender(&self) -> mpsc::UnboundedSender<ChatMessage> {
        self.message_sender.clone()
    }
    
    pub async fn next_message(&mut self) -> Option<ChatMessage> {
        self.message_receiver.recv().await
    }
    
    pub async fn run_platform<P: StreamingPlatform + Send + 'static>(
        &self,
        mut platform: P,
        channel: String,
    ) -> Result<(), P::Error> {
        platform.connect().await?;
        platform.join_channel(channel).await?;
        
        let sender = self.message_sender.clone();
        tokio::spawn(async move {
            while platform.is_connected() {
                if let Some(message) = platform.next_message().await {
                    if sender.send(message).is_err() {
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
}

impl Default for PlatformManager {
    fn default() -> Self {
        Self::new()
    }
}