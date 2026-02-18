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