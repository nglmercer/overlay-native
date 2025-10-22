# Overlay Native

Un sistema de overlay agnóstico a plataformas de streaming con soporte para múltiples conexiones WebSocket, mapeo de datos, y parseo avanzado de emotes.

## 🚀 Características Principales

### 🌐 Sistema Multiplataforma Agnóstico
- **Múltiples Plataformas**: Twitch, YouTube, Kick, Trovo, Facebook
- **Conexiones Simultáneas**: Conecta a múltiples canales de diferentes plataformas al mismo tiempo
- **Arquitectura Modular**: Sistema de plugins fácilmente extensible para nuevas plataformas

### 🎨 Sistema de Emotes Avanzado
- **Emotes de Terceros**: Soporte completo para BTTV, FFZ, 7TV
- **Cache Inteligente**: Sistema de cache con TTL y limpieza automática
- **Renderizado Multi-formato**: PNG, GIF, WebP con escalado automático
- **Detección Automática**: Parseo de emotes en tiempo real desde cualquier plataforma

### 🔄 Sistema de Mapeo de Datos
- **Normalización Unificada**: Todos los mensajes se convierten a un formato estándar
- **Transformaciones Personalizables**: Reglas de transformación configurables por plataforma
- **Filtros Avanzados**: Filtrado por usuario, contenido, nivel de acceso, etc.
- **Metadatos Enriquecidos**: Preserva información original mientras normaliza

### 🖥️ Overlay Nativo
- **Multiplataforma**: Linux (GTK) y Windows (WinAPI)
- **Ventanas Flotantes**: Overlay semi-transparente no intrusivo
- **Posicionamiento Inteligente**: Sistema de grid con posicionamiento aleatorio
- **Animaciones Suaves**: Fade in/out con duración configurable

## 📋 Requisitos del Sistema

### Comunes
- Rust 1.70+
- Memoria RAM: 512MB mínimo
- Espacio en disco: 100MB

### Linux
- GTK 3.0+
- GDK 3.0+
- X11 (o Wayland con XWayland)
- OpenSSL dev

### Windows
- Windows 10/11
- Visual Studio Build Tools 2019+
- Windows SDK 10.0+

## 🛠️ Instalación

### Desde Fuente

```bash
# Clonar el repositorio
git clone https://github.com/Brayan-724/overlay-native/
cd overlay-native

# Compilar
cargo build --release

# Ejecutar
cargo run
```

### Configuración Inicial

1. Copia `config.json.example` a `config.json`
2. Configura tus credenciales de Twitch:
```json
{
  "platforms": {
    "twitch": {
      "platform_type": "twitch",
      "enabled": true,
      "credentials": {
        "username": "TU_USERNAME",
        "oauth_token": "oauth:TU_TOKEN"
      }
    }
  }
}
```

3. Configura las conexiones deseadas:
```json
{
  "connections": [
    {
      "id": "twitch_main",
      "platform": "twitch",
      "channel": "nombre_del_canal",
      "enabled": true,
      "filters": {
        "blocked_words": ["spam", "advertisement"],
        "max_message_length": 500
      }
    }
  ]
}
```

## 📖 Configuración Avanzada

### Plataformas Soportadas

#### Twitch
```json
{
  "twitch": {
    "platform_type": "twitch",
    "enabled": true,
    "credentials": {
      "username": "tu_usuario",
      "oauth_token": "oauth:tu_token_oauth"
    },
    "settings": {
      "max_reconnect_attempts": 5,
      "reconnect_delay_ms": 5000,
      "enable_emotes": true,
      "enable_badges": true
    }
  }
}
```

#### YouTube
```json
{
  "youtube": {
    "platform_type": "youtube",
    "enabled": false,
    "credentials": {
      "client_id": "tu_client_id",
      "client_secret": "tu_client_secret",
      "api_key": "tu_api_key"
    }
  }
}
```

#### Kick
```json
{
  "kick": {
    "platform_type": "kick",
    "enabled": false,
    "credentials": {
      "username": null,
      "oauth_token": null,
      "client_id": null,
      "client_secret": null,
      "token": null
    }
  }
}
```
**🔓 No Authentication Required**: Kick allows anonymous access to public channels. You can connect to any Kick channel without providing any authentication tokens or user ID.

### Sistema de Emotes

```json
{
  "emotes": {
    "enable_global_emotes": true,
    "enable_channel_emotes": true,
    "enable_bttv": true,
    "enable_ffz": true,
    "enable_7tv": true,
    "emote_size": "medium",
    "emote_animation": true,
    "max_emotes_per_message": 50,
    "cache_enabled": true,
    "cache_ttl_hours": 24
  }
}
```

### Filtros de Mensaje

```json
{
  "filters": {
    "min_message_length": 1,
    "max_message_length": 500,
    "blocked_users": ["spamuser123"],
    "allowed_users": ["moderador"],
    "blocked_words": ["spam", "advertisement"],
    "commands_only": false,
    "subscribers_only": false,
    "vip_only": false
  }
}
```

### Configuración Visual

```json
{
  "display": {
    "font_family": "Arial",
    "font_size": 14,
    "background_color": "#1e1e1e",
    "text_color": "#ffffff",
    "username_color": "#00ff00",
    "border_radius": 8,
    "opacity": 0.9
  },
  "window": {
    "message_duration_seconds": 10,
    "max_windows": 100,
    "animation_enabled": true,
    "fade_in_duration_ms": 300,
    "fade_out_duration_ms": 500
  }
}
```

## 🏗️ Arquitectura

```
src/
├── main.rs              # Punto de entrada y orquestación principal
├── config.rs            # Sistema de configuración con validación
├── connection.rs        # Sistema de conexión y manejo de mensajes
├── platforms/           # Implementaciones de plataformas
│   ├── mod.rs          # Fábrica de plataformas y gestión
│   ├── base.rs         # Clase base abstracta para plataformas
│   ├── twitch.rs       # Implementación específica de Twitch
│   ├── youtube.rs      # Implementación específica de YouTube
│   └── kick.rs         # Implementación específica de Kick
├── emotes/             # Sistema de emotes agnóstico
│   ├── mod.rs          # Sistema principal de emotes
│   ├── cache.rs        # Cache inteligente de emotes
│   ├── parser.rs       # Parser de emotes multiplataforma
│   ├── providers.rs    # Proveedores de emotes (BTTV, FFZ, 7TV)
│   └── renderer.rs     # Renderer de imágenes de emotes
├── mapping/            # Sistema de mapeo de datos
│   ├── mod.rs          # Sistema principal de mapeo
│   ├── data_mapper.rs  # Mapeo entre formatos de plataforma
│   ├── message_transformer.rs # Transformaciones de mensajes
│   └── platform_adapter.rs    # Adaptadores de plataforma
├── window.rs           # Implementación GTK (Linux)
├── windows.rs          # Implementación WinAPI (Windows)
└── x11.rs              # Funcionalidades X11 específicas
```

## 🔌 Sistema de Plugins

El sistema está diseñado para ser fácilmente extensible:

### Añadir Nueva Plataforma

1. Crea un nuevo archivo en `src/platforms/nueva_plataforma.rs`
2. Implementa el trait `StreamingPlatform`
3. Implementa el trait `PlatformCreator`
4. Registra la plataforma en `PlatformFactory`

```rust
use async_trait::async_trait;
use crate::connection::{StreamingPlatform, ChatMessage};

pub struct NuevaPlataforma {
    // Campos específicos de la plataforma
}

#[async_trait]
impl StreamingPlatform for NuevaPlataforma {
    type Error = NuevaPlataformaError;
    
    async fn connect(&mut self) -> Result<(), Self::Error> { /* ... */ }
    async fn join_channel(&mut self, channel: String) -> Result<(), Self::Error> { /* ... */ }
    async fn next_message(&mut self) -> Option<ChatMessage> { /* ... */ }
    // ... otros métodos
}
```

### Añadir Nuevo Proveedor de Emotes

1. Implementa el trait `EmoteProvider`
2. Regístralo en `EmoteSystem`

```rust
use async_trait::async_trait;
use crate::emotes::{EmoteProvider, EmoteData, EmoteError};

pub struct NuevoProveedorEmotes;

#[async_trait]
impl EmoteProvider for NuevoProveedorEmotes {
    async fn parse_emotes(&self, message: &str, emote_data: &str) -> Result<Vec<Emote>, EmoteError> { /* ... */ }
    async fn get_channel_emotes(&self, platform: &str, channel: &str) -> Result<Vec<EmoteData>, EmoteError> { /* ... */ }
    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> { /* ... */ }
    fn provider_name(&self) -> &str { "nuevo_proveedor" }
}
```

## 🎮 Uso Avanzado

### Múltiples Conexiones

Puedes conectar a múltiples canales simultáneamente:

```json
{
  "connections": [
    {
      "id": "twitch_main",
      "platform": "twitch",
      "channel": "streamer1",
      "enabled": true
    },
    {
      "id": "youtube_secondary",
      "platform": "youtube",
      "channel": "UC...",
      "enabled": true
    },
    {
      "id": "kick_tertiary",
      "platform": "kick",
      "channel": "streamer3",
      "enabled": true
    }
  ]
}
```

### Transformaciones Personalizadas

Define reglas de transformación para cada plataforma:

```json
{
  "platforms": {
    "twitch": {
      "custom_settings": {
        "transformations": [
          {
            "field": "content",
            "operation": "replace",
            "from": "palabra_baneada",
            "to": "***"
          },
          {
            "field": "username",
            "operation": "prefix",
            "prefix": "[Twitch] "
          }
        ]
      }
    }
  }
}
```

## 🔓 Kick - Conexión Anónima

Kick permite conectarse a cualquier canal público sin necesidad de autenticación. Esta es una característica única que facilita el acceso a los chats:

### Configuración Mínima

```json
{
  "platforms": {
    "kick": {
      "platform_type": "kick",
      "enabled": true,
      "credentials": {
        "username": null,
        "oauth_token": null,
        "client_id": null,
        "client_secret": null,
        "token": null
      }
    }
  },
  "connections": [
    {
      "id": "kick_anon",
      "platform": "kick",
      "channel": "xqc",
      "enabled": true
    }
  ]
}
```

### Ejemplo de Uso en Código

```rust
use overlay_native::config::{Credentials, PlatformConfig, PlatformSettings, PlatformType};
use overlay_native::platforms::PlatformFactory;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factory = PlatformFactory::new();
    
    // Crear plataforma Kick sin autenticación
    let mut platform = factory.create_platform("kick", PlatformConfig {
        platform_type: PlatformType::Kick,
        enabled: true,
        credentials: Credentials::default(), // ← Sin autenticación!
        settings: PlatformSettings::default(),
    }).await?;
    
    // Conectar y unirse a cualquier canal público
    platform.connect().await?;
    platform.join_channel("xqc".to_string()).await?;
    
    // Escuchar mensajes
    while let Some(msg) = platform.next_message().await {
        println!("{}: {}", msg.username, msg.content);
    }
    
    Ok(())
}
```

**Ventajas de la Conexión Anónima:**
- ✅ Sin necesidad de registrar cuenta
- ✅ Sin tokens OAuth ni API keys
- ✅ Acceso instantáneo a cualquier canal público
- ✅ Ideal para testing y desarrollo
- ✅ Funciona con todos los canales públicos de Kick

## 📊 Monitorización y Logs

```json
{
  "logging": {
    "level": "info",
    "file_enabled": true,
    "console_enabled": true,
    "log_file_path": "overlay.log",
    "max_file_size_mb": 10,
    "max_files": 5
  }
}
```

Niveles de log disponibles: `trace`, `debug`, `info`, `warn`, `error`

## 🔧 Solución de Problemas

### Problemas Comunes

**No se conecta a Twitch:**
- Verifica que tu token OAuth sea válido
- Asegúrate de que el nombre de usuario sea correcto
- Revisa que el token comience con `oauth:`

**Los emotes no aparecen:**
- Verifica que el caché esté habilitado
- Revisa tu conexión a internet
- Asegúrate que los proveedores de terceros estén habilitados

**Las ventanas no aparecen:**
- En Windows, ejecuta como administrador
- En Linux, verifica que GTK esté instalado correctamente
- Revisa el monitor y configuración de grid

### Debug Mode

Ejecuta con logs detallados:

```bash
RUST_LOG=debug cargo run
```

### Verificar Configuración

```bash
cargo run -- --check-config
```

## 🤝 Contribuir

### Guía de Contribución

1. **Fork** el proyecto
2. Crea una rama: `git checkout -b feature/nueva-caracteristica`
3. Haz tus cambios siguiendo el estilo del código
4. Añade tests si es posible
5. Haz commit: `git commit -m 'Agregar nueva característica'`
6. Push: `git push origin feature/nueva-caracteristica`
7. Abre un Pull Request

### Estilo de Código

- Usar `cargo fmt` para formatear
- Usar `cargo clippy` para linting
- Documentar funciones públicas
- Añadir tests para nuevas funcionalidades

### Tests

```bash
# Ejecutar todos los tests
cargo test

# Ejecutar tests con cobertura
cargo test -- --nocapture

# Tests específicos del módulo
cargo test platforms::twitch
```

## 📄 Licencia

Este proyecto está bajo la licencia MIT. Ver `LICENSE` para más detalles.

## 🙏 Agradecimientos

- [twitch-irc](https://github.com/robotty/twitch-irc) - Cliente IRC de Twitch
- [GTK](https://www.gtk.org/) - Framework GUI para Linux
- [BetterTTV](https://betterttv.com/) - Emotes de terceros
- [FrankerFaceZ](https://www.frankerfacez.com/) - Emotes de terceros
- [7TV](https://7tv.app/) - Emotes de terceros

## 📊 Roadmap

- [ ] Soporte completo para YouTube Live Chat
- [ ] Implementación de Kick Chat
- [ ] Soporte para Trovo
- [ ] Sistema de plugins dinámicos
- [ ] Interfaz GUI para configuración
- [ ] Modo de observación (sin overlay)
- [ ] Estadísticas y analytics
- [ ] Temas y personalización avanzada
- [ ] Integración con OBS

## 📞 Contacto

- GitHub Issues: [Reportar problemas](https://github.com/Brayan-724/overlay-native/issues)
- Discord: [Servidor de la comunidad](https://discord.gg/...)

---

**Overlay Native** - Hecho con ❤️ por la comunidad de streaming