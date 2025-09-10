# Guía de Instalación

Esta guía describe cómo preparar el entorno y compilar Overlay Native en Linux y Windows.

## Requisitos Generales
- Rust (1.70+ recomendado), instalado con rustup
- Git
- pkg-config (recomendado en Linux)

## Linux

### Dependencias del sistema
Instala las dependencias de desarrollo necesarias para compilar GTK (usado por la implementación Linux):

- Debian/Ubuntu:
  - sudo apt update && sudo apt install -y build-essential pkg-config libgtk-3-dev libssl-dev
- Fedora:
  - sudo dnf install -y @development-tools pkgconfig gtk3-devel openssl-devel
- Arch/Manjaro:
  - sudo pacman -S --needed base-devel pkgconf gtk3 openssl

Notas:
- Requiere un servidor X11 en ejecución (el backend integra utilidades específicas de X11).
- Wayland: el proyecto utiliza utilidades X11; en sesiones Wayland puede requerir XWayland.

### Compilación y ejecución
```
git clone https://github.com/Brayan-724/overlay-native.git
cd overlay-native
cargo run
```

Para binario optimizado:
```
cargo build --release
```

## Windows

### Dependencias del sistema
- Windows 10/11
- Rust (rustup)
- Toolchain de compilación (elige una):
  - Opción A: Visual Studio Build Tools o Visual Studio Community con la carga de trabajo “Desktop development with C++”
  - Opción B: MSYS2 + MinGW (si prefieres toolchain MinGW)

Notas:
- La implementación Windows usa WinAPI nativo. No necesitas GTK para compilar en Windows (la parte GTK es para Linux).

### Compilación y ejecución
```
git clone https://github.com/Brayan-724/overlay-native.git
cd overlay-native
cargo run
```

Para binario optimizado:
```
cargo build --release
```

## Configuración
Actualmente el canal de Twitch está hardcodeado en el código (main.rs) y apunta a "mictia00". En una futura versión se añadirá un archivo de configuración.

## Solución de Problemas
- Error de GTK en Linux: verifica que instalaste libgtk-3-dev/gtk3-devel (según tu distribución) y pkg-config.
- Enlaces TLS/SSL: instala los headers de OpenSSL (libssl-dev/openssl-devel) si fuese necesario.
- Fuentes/DPI en Windows: si observas renderizado borroso, verifica la configuración de escala de pantalla. 
- Si el compilador no encuentra pkg-config en Linux, instálalo (pkg-config/pkgconf).