# Arquitectura de Overlay Native

Overlay Native es una aplicación multiplataforma de overlay que recibe mensajes de plataformas de streaming a través de una capa de transporte agnóstica (IPC/WebSocket). La arquitectura separa completamente la lógica de plataformas del renderizado, permitiendo que cualquier fuente externa envíe mensajes validados.

## Arquitectura General

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    EXTERNAL PLATFORM SERVICES                           │
│   (Twitch Bridge, Kick Bridge, YouTube Bridge, Custom Clients)         │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         TRANSPORT LAYER                                 │
│                    (IPC, WebSocket, HTTP API)                           │
│                                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌──────────────┐               │
│  │    IPC      │    │  WebSocket  │    │  Validation  │               │
│  │  (Local)    │    │  (Remote)   │    │   (Schema)   │               │
│  └──────┬──────┘    └──────┬──────┘    └──────┬───────┘               │
│         └──────────────────┼──────────────────┘                        │
└────────────────────────────┼────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            CORE LAYER                                   │
│                   (Platform-Agnostic Rendering)                         │
│                                                                         │
│  • Message types (ChatMessageElement, GiftElement, EmoteElement)       │
│  • Core renderer with filtering and queuing                            │
│  • Configuration management                                             │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          RENDER LAYER                                   │
│                   (Platform-Specific Rendering)                         │
│                                                                         │
│  ┌────────────────────────┐    ┌────────────────────────┐             │
│  │      Linux/GTK         │    │     Windows/Win32      │             │
│  │  • GTK Window          │    │  • HWND Window         │             │
│  │  • X11 Integration     │    │  • GDI Rendering       │             │
│  │  • Progress Bar        │    │  • Progress Bar        │             │
│  └────────────────────────┘    └────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────────┘
```

## Estructura del Proyecto

```
src/
├── main.rs              # Punto de entrada
├── lib.rs               # Exports de la librería con documentación
│
├── core/                # ⭐ Núcleo agnóstico de plataforma
│   ├── mod.rs           # Exports del core
│   ├── config.rs        # Configuración del renderer
│   ├── message.rs       # Tipos de mensajes agnósticos
│   └── renderer.rs      # Motor de renderizado con filtrado
│
├── transport/           # ⭐ Capa de transporte (API de entrada)
│   ├── mod.rs           # Exports y documentación
│   ├── bridge.rs        # Puente transporte → core
│   ├── schema.rs        # Esquema de validación de mensajes
│   ├── ipc.rs           # Servidor IPC (Unix sockets/Named pipes)
│   └── websocket.rs     # Servidor WebSocket
│
├── render/              # ⭐ NUEVO: Renderizado específico de SO
│   ├── mod.rs           # Trait PlatformWindow y configuración
│   ├── gtk.rs           # Implementación GTK para Linux
│   └── win32.rs         # Implementación Win32 para Windows
│
├── config.rs            # Configuración global legacy
├── connection.rs        # Tipos de conexión legacy
├── platforms/           # Gestor de credenciales (legacy)
├── emotes/              # Sistema de emotes
├── mapping/             # Transformación de datos
│
├── window.rs            # (Legacy) GTK implementation
├── windows.rs           # (Legacy) WinAPI implementation
└── x11.rs               # Utilidades X11 para Linux
```

## Flujo de Datos

```
Plataforma Externa
       │
       │ (JSON via WebSocket/IPC)
       ▼
┌──────────────────┐
│  WebSocket/IPC   │
│    Servidor      │
└────────┬─────────┘
         │
         │ WsEvent
         ▼
┌──────────────────┐     ┌──────────────────┐
│  Transport       │     │  Validación     │
│  Bridge          │────►│  (Schema)        │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         │ IncomingMessage        │ ChatMessagePayload
         │ (validado)             │ (validado)
         ▼                        ▼
┌──────────────────┐     ┌──────────────────┐
│  Core            │◄────│  Conversión       │
│  Renderer        │     │  (Into<>)        │
└────────┬─────────┘     └──────────────────┘
         │
         │ OverlayElement
         ▼
┌──────────────────┐
│  Window          │
│  (GTK/WinAPI)    │
└──────────────────┘
```

## Módulos y Responsabilidades

### Core (`src/core/`)

El núcleo es **completamente agnóstico** a cualquier plataforma de streaming. Solo conoce tipos de datos genéricos.

- **config.rs**: Configuración del renderer (ventanas, animaciones, display)
- **message.rs**: Tipos de mensajes unificados (`ChatMessageElement`, `GiftElement`, `EmoteElement`)
- **renderer.rs**: Motor de renderizado que recibe elementos y los muestra

### Transporte (`src/transport/`)

Maneja la comunicación con servicios externos que actúan como bridges de plataformas.

- **schema.rs**: Define el esquema de validación de mensajes entrantes
- **websocket.rs**: Servidor WebSocket para conexiones remotas
- **ipc.rs**: Servidor IPC (Unix Domain Sockets/Named Pipes) para local
- **bridge.rs**: Puente que conecta transporte con el renderer

### Plataforma Externa (No incluida)

Los bridges de plataformas (Twitch, Kick, YouTube, etc.) son servicios externos que se conectan al overlay vía WebSocket o IPC. Deben enviar mensajes que cumplan con el esquema definido en `transport/schema.rs`.

## Esquema de Mensajes

Los mensajes deben enviarse como JSON con la estructura definida en `src/transport/schema.rs`:

```json
// Mensaje de chat
{
  "type": "chat_message",
  "data": {
    "id": "msg_123",
    "username": "user123",
    "display_name": "User123",
    "content": "Hello world!",
    "user_color": "#FF0000",
    "emotes": [...],
    "badges": [...]
  }
}

// Gift/Subscription
{
  "type": "gift",
  "data": {
    "from_user": "gifter",
    "to_user": "recipient",
    "gift_type": "subscription",
    "amount": 1,
    "tier": "Tier 1"
  }
}

// Emote
{
  "type": "emote",
  "data": {
    "id": "emote_123",
    "name": "Kappa",
    "url": "https://...",
    "is_animated": false
  }
}
```

## Configuración

La configuración en `config.json` ahora incluye una sección de transporte:

```json
{
  "transport": {
    "websocket_enabled": true,
    "websocket_bind": "127.0.0.1:9001",
    "ipc_enabled": true,
    "ipc_socket_path": "/tmp/overlay-native.sock",
    "max_connections": 100,
    "strict_validation": true
  },
  "connections": [...],
  "window": {...},
  "emotes": {...}
}
```

## Uso con Bridges Externos

Para conectar plataformas, usa bridges externos que se comuniquen via WebSocket:

1. **Inicia el overlay**: `cargo run`
2. **Conecta un bridge**: Envía mensajes JSON al `websocket_bind` especificado
3. **Ejemplo con WebSocket**:
   ```javascript
   const ws = new WebSocket('ws://127.0.0.1:9001');
   ws.send(JSON.stringify({
     type: 'chat_message',
     data: {
       username: 'test_user',
       content: 'Hello from external bridge!'
     }
   }));
   ```

## Beneficios de la Arquitectura

1. **Agnóstico**: El renderer no conoce ni le importa de dónde vienen los mensajes
2. **Flexible**: Cualquier plataforma puede conectarse via WebSocket/IPC
3. **Validado**: Todos los mensajes pasan por validación de esquema
4. **Escalable**: Múltiples bridges pueden conectarse simultáneamente
5. **Mantenible**: Lógica de plataformas separada del renderizado

## Dependencias Principales

- **tokio**: Runtime asíncrono
- **gtk/gdk/pango/glib**: Stack de GUI para Linux
- **winapi**: Interacciones nativas en Windows
- **x11rb/gdkx11**: Integración X11
- **reqwest**: Descarga de recursos
- **serde**: Serialización de JSON
- **tokio-tungstenite**: WebSocket server

## Ciclo de Vida de Ventanas

- Creación al recibir un elemento del renderer
- Posicionamiento basado en configuración
- Barra de progreso hasta cierre automático (~10s por defecto)
- Liberación de recursos al destruir
