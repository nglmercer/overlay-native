/// Transporte IPC (Inter-Process Communication) para recibir mensajes localmente
///
/// En Linux/macOS: usa Unix Domain Sockets (UDS)
/// En Windows: usa Named Pipes
///
/// Es la forma más eficiente para comunicación local (misma máquina).
/// Ideal cuando el bridge de plataforma corre como proceso separado.
use tokio::sync::mpsc;

use crate::transport::schema::IncomingMessage;
use crate::transport::websocket::WsEvent; // Reutilizamos el mismo tipo de evento

/// Configuración del servidor IPC
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Ruta del socket (Linux/macOS) o nombre del pipe (Windows)
    pub socket_path: String,
}

impl Default for IpcConfig {
    fn default() -> Self {
        #[cfg(unix)]
        return Self {
            socket_path: "/tmp/overlay-native.sock".to_string(),
        };

        #[cfg(windows)]
        return Self {
            socket_path: r"\\.\pipe\overlay-native".to_string(),
        };
    }
}

/// Servidor IPC - Unix Domain Sockets (Linux/macOS)
#[cfg(unix)]
pub struct IpcServer {
    config: IpcConfig,
    event_tx: mpsc::UnboundedSender<WsEvent>,
}

#[cfg(unix)]
impl IpcServer {
    pub fn new(config: IpcConfig, event_tx: mpsc::UnboundedSender<WsEvent>) -> Self {
        Self { config, event_tx }
    }

    /// Inicia el servidor IPC
    pub async fn start(self) -> anyhow::Result<()> {
        use tokio::net::UnixListener;

        // Eliminar socket anterior si existe
        let _ = std::fs::remove_file(&self.config.socket_path);

        let listener = UnixListener::bind(&self.config.socket_path).map_err(|e| {
            anyhow::anyhow!(
                "No se pudo crear Unix socket en '{}': {}",
                self.config.socket_path,
                e
            )
        })?;

        println!(
            "[IPC] 🔌 Unix socket escuchando en: {}",
            self.config.socket_path
        );
        println!("[IPC] 📋 Protocolo: JSON por línea (newline-delimited JSON)");
        println!(
            "[IPC] 💡 Envía: echo '{{\"type\":\"ping\"}}' | nc -U {}",
            self.config.socket_path
        );

        let event_tx = self.event_tx;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_unix_connection(stream, tx).await {
                            eprintln!("[IPC] Error en conexión Unix: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[IPC] Error aceptando conexión: {}", e);
                }
            }
        }
    }
}

/// Maneja una conexión Unix Domain Socket
#[cfg(unix)]
async fn handle_unix_connection(
    stream: tokio::net::UnixStream,
    event_tx: mpsc::UnboundedSender<WsEvent>,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    println!("[IPC] 📥 Cliente IPC conectado");

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        match IncomingMessage::parse_and_validate(&line) {
            Ok(msg) => {
                // Responder con ACK
                if let IncomingMessage::ChatMessage(ref payload) = msg {
                    let id = payload.get_or_generate_id();
                    let ack = format!("{{\"type\":\"ack\",\"data\":{{\"id\":\"{}\"}}}}\n", id);
                    let _ = writer.write_all(ack.as_bytes()).await;
                }

                if let Err(e) = event_tx.send(WsEvent::Message(Box::new(msg))) {
                    eprintln!("[IPC] Error reenviando mensaje: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("[IPC] Mensaje inválido: {}", e);
                let error = format!(
                    "{{\"type\":\"error\",\"data\":{{\"code\":\"VALIDATION_ERROR\",\"message\":\"{}\"}}}}\n",
                    e.to_string().replace('"', "\\\"")
                );
                let _ = writer.write_all(error.as_bytes()).await;
            }
        }
    }

    println!("[IPC] 👋 Cliente IPC desconectado");
    Ok(())
}

/// Servidor IPC - Named Pipes (Windows)
#[cfg(windows)]
pub struct IpcServer {
    config: IpcConfig,
    event_tx: mpsc::UnboundedSender<WsEvent>,
}

#[cfg(windows)]
impl IpcServer {
    pub fn new(config: IpcConfig, event_tx: mpsc::UnboundedSender<WsEvent>) -> Self {
        Self { config, event_tx }
    }

    pub async fn start(self) -> anyhow::Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ServerOptions;

        println!(
            "[IPC] 🔌 Named Pipe escuchando en: {}",
            self.config.socket_path
        );
        println!("[IPC] 📋 Protocolo: JSON por línea (newline-delimited JSON)");

        let event_tx = self.event_tx;
        let pipe_name = self.config.socket_path.clone();

        loop {
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&pipe_name)
                .map_err(|e| anyhow::anyhow!("Error creando Named Pipe: {}", e))?;

            // Esperar conexión
            server
                .connect()
                .await
                .map_err(|e| anyhow::anyhow!("Error esperando conexión en Named Pipe: {}", e))?;

            let tx = event_tx.clone();
            let pipe_name_clone = pipe_name.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_named_pipe_connection(server, tx).await {
                    eprintln!("[IPC] Error en conexión Named Pipe: {}", e);
                }
            });
        }
    }
}

/// Maneja una conexión Named Pipe (Windows)
#[cfg(windows)]
async fn handle_named_pipe_connection(
    pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    event_tx: mpsc::UnboundedSender<WsEvent>,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = tokio::io::split(pipe);
    let mut lines = BufReader::new(reader).lines();

    println!("[IPC] 📥 Cliente Named Pipe conectado");

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        match IncomingMessage::parse_and_validate(&line) {
            Ok(msg) => {
                if let IncomingMessage::ChatMessage(ref payload) = msg {
                    let id = payload.get_or_generate_id();
                    let ack = format!("{{\"type\":\"ack\",\"data\":{{\"id\":\"{}\"}}}}\n", id);
                    let _ = writer.write_all(ack.as_bytes()).await;
                }

                if let Err(e) = event_tx.send(WsEvent::Message(Box::new(msg))) {
                    eprintln!("[IPC] Error reenviando mensaje: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("[IPC] Mensaje inválido: {}", e);
                let error = format!(
                    "{{\"type\":\"error\",\"data\":{{\"code\":\"VALIDATION_ERROR\",\"message\":\"{}\"}}}}\n",
                    e.to_string().replace('"', "\\\"")
                );
                let _ = writer.write_all(error.as_bytes()).await;
            }
        }
    }

    println!("[IPC] 👋 Cliente Named Pipe desconectado");
    Ok(())
}
