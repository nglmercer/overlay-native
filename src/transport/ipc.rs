use tokio::sync::mpsc;
use crate::transport::schema::IncomingMessage;
use crate::transport::websocket::WsEvent;

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub socket_path: String,
}

impl Default for IpcConfig {
    fn default() -> Self {
        #[cfg(unix)]
        return Self { socket_path: "/tmp/overlay-native.sock".to_string() };
        #[cfg(windows)]
        return Self { socket_path: r"\\.\pipe\overlay-native".to_string() };
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
        #[cfg(unix)]
        let _ = self.start_unix().await;
        #[cfg(windows)]
        let _ = self.start_windows().await;
        Ok(())
    }

    #[cfg(unix)]
    async fn start_unix(self) -> anyhow::Result<()> {
        use tokio::net::UnixListener;
        let _ = std::fs::remove_file(&self.config.socket_path);
        let listener = UnixListener::bind(&self.config.socket_path)?;
        println!("[IPC] 🔌 Unix socket listening on: {}", self.config.socket_path);

        let event_tx = self.event_tx;
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_unix_connection(stream, tx).await {
                        eprintln!("[IPC] Unix connection error: {}", e);
                    }
                });
            }
        }
    }

    #[cfg(windows)]
    async fn start_windows(self) -> anyhow::Result<()> {
        use tokio::net::windows::named_pipe::ServerOptions;
        println!("[IPC] 🔌 Named Pipe listening on: {}", self.config.socket_path);
        let event_tx = self.event_tx;
        let pipe_name = self.config.socket_path.clone();
        loop {
            let server = ServerOptions::new().first_pipe_instance(false).create(&pipe_name)?;
            server.connect().await?;
            let tx = event_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_named_pipe_connection(server, tx).await {
                    eprintln!("[IPC] Named Pipe error: {}", e);
                }
            });
        }
    }
}

#[cfg(unix)]
async fn handle_unix_connection(stream: tokio::net::UnixStream, event_tx: mpsc::UnboundedSender<WsEvent>) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        process_line(&line, &mut writer, &event_tx).await?;
    }
    Ok(())
}

#[cfg(windows)]
async fn handle_named_pipe_connection(pipe: tokio::net::windows::named_pipe::NamedPipeServer, event_tx: mpsc::UnboundedSender<WsEvent>) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let (reader, mut writer) = tokio::io::split(pipe);
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        process_line(&line, &mut writer, &event_tx).await?;
    }
    Ok(())
}

async fn process_line<W: tokio::io::AsyncWrite + Unpin>(
    line: &str,
    writer: &mut W,
    event_tx: &mpsc::UnboundedSender<WsEvent>
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    match IncomingMessage::parse_and_validate(line) {
        Ok(msg) => {
            if let IncomingMessage::ChatMessage(ref payload) = msg {
                let id = payload.get_or_generate_id();
                let ack = format!("{{\"type\":\"ack\",\"data\":{{\"id\":\"{}\"}}}}\n", id);
                let _ = writer.write_all(ack.as_bytes()).await;
            }
            let _ = event_tx.send(WsEvent::Message(Box::new(msg)));
        }
        Err(e) => {
            let error = format!("{{\"type\":\"error\",\"data\":{{\"code\":\"VALIDATION_ERROR\",\"message\":\"{}\"}}}}\n", e.to_string().replace('"', "\\\""));
            let _ = writer.write_all(error.as_bytes()).await;
        }
    }
    Ok(())
}
