use crate::transport::schema::IncomingMessage;
use crate::transport::websocket::WsEvent;
use ipc_lib::{CommunicationMessage, ProtocolType, SingleInstanceApp};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub socket_path: String,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            socket_path: "overlay-native".to_string(),
        }
    }
}

pub struct IpcServer {
    config: IpcConfig,
    event_tx: mpsc::UnboundedSender<WsEvent>,
}

impl IpcServer {
    pub fn new(config: IpcConfig, event_tx: mpsc::UnboundedSender<WsEvent>) -> Self {
        Self { config, event_tx }
    }

    pub async fn start(self) -> anyhow::Result<()> {
        let identifier = self.config.socket_path.clone();
        let event_tx = self.event_tx.clone();

        println!("[IPC] 🚀 Initializing IPC Server with identifier: {}", identifier);

        let mut app = SingleInstanceApp::new(&identifier).on_message(move |msg: CommunicationMessage| {
            // Log message reception
            println!("[IPC] 📩 Received msg type: '{}' from '{}'", msg.message_type, msg.source_id);

            // We support two ways of receiving messages:
            // 1. message_type is the schema type (chat_message, etc.) and payload is the data
            // 2. payload itself is the full IncomingMessage JSON
            
            let incoming_result = if msg.message_type == "chat_message" || msg.message_type == "gift" || msg.message_type == "image" {
                // Wrap payload into a tagged JSON for IncomingMessage
                let wrapped = serde_json::json!({
                    "type": msg.message_type,
                    "data": msg.payload
                });
                serde_json::from_value::<IncomingMessage>(wrapped)
            } else {
                // Try parsing the payload directly as IncomingMessage
                serde_json::from_value::<IncomingMessage>(msg.payload.clone())
            };

            match incoming_result {
                Ok(incoming) => {
                    // Forward to the bridge
                    if let Err(e) = event_tx.send(WsEvent::Message(Box::new(incoming.clone()))) {
                        eprintln!("[IPC] ❌ Failed to forward message to bridge: {}", e);
                    }

                    // Handle Ack if it's a chat message (maintaining compatibility with overlay-native expectation)
                    if let IncomingMessage::ChatMessage(ref p) = incoming {
                        let id = p.get_or_generate_id();
                        return Some(CommunicationMessage::new("ack", serde_json::json!({ "id": id })));
                    }

                    Some(CommunicationMessage::new("success", serde_json::json!({ "status": "processed" })))
                }
                Err(e) => {
                    eprintln!("[IPC] ⚠️ Failed to parse incoming message: {}", e);
                    Some(CommunicationMessage::new("error", serde_json::json!({
                        "code": "VALIDATION_ERROR",
                        "message": e.to_string()
                    })))
                }
            }
        });

        // Configure protocol based on platform
        #[cfg(unix)]
        { 
            app = app.with_protocol(ProtocolType::UnixSocket); 
        }
        #[cfg(windows)]
        { 
            app = app.with_protocol(ProtocolType::FileBased); // Use FileBased on Windows until NamedPipe is ready in ipc_lib
        }

        match app.enforce_single_instance().await {
            Ok(true) => {
                println!("[IPC] ✅ Server is now primary and listening for messages");
                // Keep the server alive. SingleInstanceApp manages the background tasks.
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            }
            Ok(false) => {
                println!("[IPC] ℹ️ Another instance is already running. This instance will act as a client.");
                // In the context of overlay-native, we usually want only one server.
                Err(anyhow::anyhow!("Overlay-native is already running (Single instance enforcement)"))
            }
            Err(e) => {
                Err(anyhow::anyhow!("Failed to start IPC server: {}", e))
            }
        }
    }
}
