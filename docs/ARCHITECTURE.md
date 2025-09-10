# Arquitectura de Overlay Native

Overlay Native es una aplicación multiplataforma que consume Twitch IRC usando el crate twitch-irc y renderiza overlays nativos por plataforma. La arquitectura separa la lógica de negocio de las implementaciones específicas de plataforma.

## Estructura del Proyecto

```
src/
├── main.rs          # Punto de entrada y gestión del cliente Twitch IRC (twitch-irc)
├── connection.rs    # (Reservado) Abstracción futura de conexión
├── window.rs        # Implementación GTK (Linux)
├── windows.rs       # Implementación WinAPI (Windows)
└── x11.rs           # Funcionalidades específicas de X11
```

## Módulos y Responsabilidades

- main.rs: Inicializa el runtime (Tokio), configura y arranca el cliente de Twitch usando twitch-irc, recibe eventos de chat y crea ventanas overlay por mensaje.
- connection.rs: Espacio reservado para una futura abstracción (actualmente sin lógica productiva).
- window.rs: Crea y administra ventanas GTK, con etiqueta, barra de progreso y carga básica de emotes.
- windows.rs: Administra ventanas WinAPI, incluyendo creación/destrucción y actualización del progreso.
- x11.rs: Utilidades X11 (e.g., propiedades de ventana) para mejorar integración en Linux.

## Flujo de Datos

```
Twitch (IRC) -> twitch-irc (cliente) -> main.rs (handler) -> {window.rs | windows.rs} -> Overlay en pantalla
```

## Cliente IRC (twitch-irc)

En lugar de implementar el protocolo IRC manualmente, se usa el crate twitch-irc para gestionar la conexión, autenticación y parsing de mensajes. Esto reduce complejidad y errores.

Ejemplo conceptual (simplificado):

```rust
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::TwitchIRCClient;

let config = ClientConfig::default();
let credentials = StaticLoginCredentials::new("justinfan12345".to_owned(), None);
let (mut incoming_messages, client) = TwitchIRCClient::new(config, credentials);
client.join("mictia00".to_owned()).unwrap();

while let Some(message) = incoming_messages.recv().await {
    // Crear overlay con window.rs o windows.rs
}
```

## Ciclo de Vida de Ventanas

- Creación al recibir un mensaje.
- Posicionamiento aleatorio y transparencia.
- Barra de progreso hasta cierre automático (~10s).
- Liberación de recursos al destruir.

## Concurrencia

- Tokio para tareas asíncronas (escucha de mensajes, timers).
- Canales/eventos entre el manejador de chat y el renderizador de ventanas.

## Renderizado y Emotes

- GTK (Linux) y WinAPI (Windows).
- Emotes: soporte básico/experimental; la carga y renderizado están parcialmente implementados y pueden variar según plataforma.

## Configuración

- Canal por defecto hardcodeado actualmente: "mictia00" en main.rs.
- Futuro: archivo de configuración para credenciales/canales.

## Dependencias Principales

- twitch-irc: cliente IRC de Twitch.
- tokio: runtime asíncrono.
- gtk/gdk/pango/glib: stack de GUI para Linux.
- winapi: interacciones nativas en Windows.
- x11rb/gdkx11: integración X11.
- reqwest: descarga de recursos (e.g., emotes).
- rand: utilidades aleatorias para posicionamiento.

## Depuración y Métricas

- Logs informativos al conectar a Twitch y al crear ventanas.
- Métricas básicas: número de ventanas activas, tiempos de vida.

## Roadmap (extracto)

- Configuración externa.
- Mejorar pipeline de emotes (cache, animados).
- Tests de integración por plataforma.