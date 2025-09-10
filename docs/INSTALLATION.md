# Guía de Instalación y Compilación

## 📋 Requisitos Generales

### Rust
- **Versión mínima**: Rust 1.70+
- **Toolchain recomendado**: stable
- **Componentes**: rustc, cargo, rustfmt, clippy

```bash
# Instalar Rust (si no está instalado)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Actualizar Rust
rustup update

# Verificar instalación
rustc --version
cargo --version
```

## 🐧 Instalación en Linux

### Distribuciones Soportadas
- Ubuntu 20.04+ / Debian 11+
- Fedora 35+ / RHEL 8+
- Arch Linux
- openSUSE Leap 15.3+

### Dependencias del Sistema

#### Ubuntu/Debian
```bash
# Actualizar repositorios
sudo apt update

# Instalar dependencias de desarrollo
sudo apt install -y \
    build-essential \
    pkg-config \
    libgtk-3-dev \
    libgdk-pixbuf2.0-dev \
    libatk1.0-dev \
    libcairo-gobject2 \
    libpango1.0-dev \
    libglib2.0-dev \
    libx11-dev \
    libxext-dev

# Dependencias opcionales para mejor experiencia
sudo apt install -y \
    fonts-dejavu-core \
    fonts-liberation \
    compton  # o picom para composición
```

#### Fedora/RHEL/CentOS
```bash
# Instalar dependencias de desarrollo
sudo dnf install -y \
    gcc \
    gcc-c++ \
    pkg-config \
    gtk3-devel \
    gdk-pixbuf2-devel \
    atk-devel \
    cairo-gobject-devel \
    pango-devel \
    glib2-devel \
    libX11-devel \
    libXext-devel

# Dependencias opcionales
sudo dnf install -y \
    dejavu-fonts \
    liberation-fonts \
    picom
```

#### Arch Linux
```bash
# Instalar dependencias
sudo pacman -S \
    base-devel \
    pkg-config \
    gtk3 \
    gdk-pixbuf2 \
    atk \
    cairo \
    pango \
    glib2 \
    libx11 \
    libxext

# Dependencias opcionales
sudo pacman -S \
    ttf-dejavu \
    ttf-liberation \
    picom
```

#### openSUSE
```bash
# Instalar dependencias
sudo zypper install -y \
    gcc \
    gcc-c++ \
    pkg-config \
    gtk3-devel \
    gdk-pixbuf-devel \
    atk-devel \
    cairo-devel \
    pango-devel \
    glib2-devel \
    libX11-devel \
    libXext-devel
```

### Verificación de Dependencias
```bash
# Verificar GTK
pkg-config --modversion gtk+-3.0

# Verificar GDK
pkg-config --modversion gdk-3.0

# Verificar X11
echo $DISPLAY  # Debe mostrar algo como :0 o :1
```

## 🪟 Instalación en Windows

### Requisitos del Sistema
- **Windows 10** (versión 1903+) o **Windows 11**
- **Arquitectura**: x64 (recomendado) o x86
- **Espacio libre**: ~2GB para herramientas de desarrollo

### Opción 1: Visual Studio Community (Recomendado)
```powershell
# Descargar e instalar Visual Studio Community
# https://visualstudio.microsoft.com/vs/community/

# Durante la instalación, seleccionar:
# - "Desarrollo para el escritorio con C++"
# - Windows 10/11 SDK (última versión)
# - MSVC v143 compiler toolset
# - CMake tools for Visual Studio (opcional)
```

### Opción 2: Build Tools Standalone
```powershell
# Descargar Build Tools for Visual Studio
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022

# Instalar con componentes mínimos:
# - MSVC v143 - VS 2022 C++ x64/x86 build tools
# - Windows 10/11 SDK
```

### Configurar Rust para Windows
```powershell
# Instalar Rust con toolchain MSVC
Rustup-init.exe

# Durante la instalación, seleccionar:
# 1) Proceed with installation (default)
# Default host triple: x86_64-pc-windows-msvc

# Verificar instalación
rustc --version --verbose
cargo --version

# Instalar target adicional si es necesario
rustup target add x86_64-pc-windows-msvc
```

### Verificación de Dependencias
```powershell
# Verificar compilador C++
cl.exe

# Verificar Windows SDK
dir "C:\Program Files (x86)\Windows Kits\10\Include"

# Verificar variables de entorno
echo $env:INCLUDE
echo $env:LIB
```

## 🚀 Compilación del Proyecto

### Clonar el Repositorio
```bash
# Clonar desde Git
git clone <repository-url>
cd overlay-native

# O descargar y extraer ZIP
# wget <repository-zip-url>
# unzip overlay-native.zip
# cd overlay-native
```

### Compilación en Modo Debug
```bash
# Compilar (modo debug)
cargo build

# Ejecutar directamente
cargo run

# Ejecutar con logs detallados
RUST_LOG=debug cargo run
```

### Compilación en Modo Release

#### Linux
```bash
# Compilación optimizada
cargo build --release

# Con optimizaciones específicas del CPU
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Compilación estática (opcional)
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl
```

#### Windows
```powershell
# Compilación optimizada
cargo build --release

# Compilación estática (recomendado para distribución)
$env:RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc

# Con optimizaciones específicas
$env:RUSTFLAGS="-C target-cpu=native -C target-feature=+crt-static"
cargo build --release
```

### Ubicación de Binarios
```bash
# Modo debug
./target/debug/overlay-native      # Linux
.\target\debug\overlay-native.exe  # Windows

# Modo release
./target/release/overlay-native      # Linux
.\target\release\overlay-native.exe  # Windows
```

## 🧪 Testing y Verificación

### Tests Unitarios
```bash
# Ejecutar todos los tests
cargo test

# Tests con output detallado
cargo test -- --nocapture

# Tests específicos
cargo test window
```

### Verificación Manual

#### Linux
```bash
# Verificar dependencias en runtime
ldd target/release/overlay-native

# Verificar que X11 está disponible
echo $DISPLAY
xdpyinfo | head

# Test básico
./target/release/overlay-native
```

#### Windows
```powershell
# Verificar dependencias
dumpbin /dependents target\release\overlay-native.exe

# Test básico
.\target\release\overlay-native.exe
```

### Verificación de Funcionalidad
1. **Conexión a Twitch**: Verificar logs de conexión IRC
2. **Creación de ventanas**: Enviar mensaje de prueba al chat
3. **Renderizado**: Verificar que texto y barra de progreso aparecen
4. **Transparencia**: Verificar que las ventanas son semi-transparentes
5. **Auto-cierre**: Verificar que ventanas se cierran después de 10s

## 🔧 Configuración Avanzada

### Variables de Entorno

#### Para Desarrollo
```bash
# Linux/macOS
export RUST_LOG=debug
export RUST_BACKTRACE=1
export GTK_DEBUG=all  # Solo Linux

# Windows PowerShell
$env:RUST_LOG="debug"
$env:RUST_BACKTRACE="1"
```

#### Para Producción
```bash
# Optimizaciones de compilación
export RUSTFLAGS="-C target-cpu=native -C opt-level=3"

# Windows - compilación estática
$env:RUSTFLAGS="-C target-feature=+crt-static -C target-cpu=native"
```

### Configuración de Cargo

Crear `.cargo/config.toml`:
```toml
[build]
# Configuración global
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-pc-windows-msvc]
# Windows específico
rustflags = ["-C", "target-feature=+crt-static"]

[target.x86_64-unknown-linux-gnu]
# Linux específico
rustflags = ["-C", "link-arg=-Wl,--strip-all"]
```

## 🐛 Solución de Problemas

### Errores Comunes de Compilación

#### Linux: "pkg-config not found"
```bash
# Ubuntu/Debian
sudo apt install pkg-config

# Fedora/RHEL
sudo dnf install pkgconf-pkg-config

# Arch
sudo pacman -S pkgconf
```

#### Linux: "gtk-3.0 not found"
```bash
# Verificar instalación
pkg-config --list-all | grep gtk

# Reinstalar si es necesario
sudo apt install --reinstall libgtk-3-dev
```

#### Windows: "link.exe not found"
```powershell
# Verificar Visual Studio Build Tools
where link.exe

# Si no se encuentra, reinstalar Build Tools
# O configurar manualmente:
$env:PATH += ";C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.xx.xxxxx\bin\Hostx64\x64"
```

#### Windows: "Windows SDK not found"
```powershell
# Verificar SDK instalado
dir "C:\Program Files (x86)\Windows Kits\10\Include"

# Configurar variables si es necesario
$env:INCLUDE = "C:\Program Files (x86)\Windows Kits\10\Include\10.0.xxxxx.x\um;$env:INCLUDE"
$env:LIB = "C:\Program Files (x86)\Windows Kits\10\Lib\10.0.xxxxx.x\um\x64;$env:LIB"
```

### Errores de Runtime

#### Linux: "cannot open display"
```bash
# Verificar X11
echo $DISPLAY

# Si está vacío, configurar
export DISPLAY=:0

# Para SSH con X forwarding
ssh -X usuario@servidor
```

#### Linux: "Gtk-WARNING: cannot open display"
```bash
# Verificar permisos X11
xhost +local:

# O para usuario específico
xhost +SI:localuser:$(whoami)
```

#### Windows: "Failed to create window"
```powershell
# Ejecutar como administrador si es necesario
# O verificar permisos de UAC

# Verificar que no hay antivirus bloqueando
# Agregar excepción si es necesario
```

### Performance Issues

#### Compilación Lenta
```bash
# Usar compilación paralela
export CARGO_BUILD_JOBS=4  # o número de cores

# Usar cache de compilación
cargo install sccache
export RUSTC_WRAPPER=sccache
```

#### Runtime Lento
```bash
# Verificar modo release
cargo build --release

# Profiling si es necesario
cargo install flamegraph
cargo flamegraph
```

## 📦 Distribución

### Crear Paquete Portable

#### Linux
```bash
# Compilar estáticamente
cargo build --release --target x86_64-unknown-linux-musl

# Crear directorio de distribución
mkdir -p dist/overlay-native-linux
cp target/x86_64-unknown-linux-musl/release/overlay-native dist/overlay-native-linux/
cp README.md dist/overlay-native-linux/
cp docs/ dist/overlay-native-linux/ -r

# Crear tarball
tar -czf overlay-native-linux.tar.gz -C dist overlay-native-linux
```

#### Windows
```powershell
# Compilar estáticamente
$env:RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release

# Crear directorio de distribución
New-Item -ItemType Directory -Path "dist\overlay-native-windows" -Force
Copy-Item "target\release\overlay-native.exe" "dist\overlay-native-windows\"
Copy-Item "README.md" "dist\overlay-native-windows\"
Copy-Item "docs" "dist\overlay-native-windows\" -Recurse

# Crear ZIP
Compress-Archive -Path "dist\overlay-native-windows" -DestinationPath "overlay-native-windows.zip"
```

### Verificación de Distribución
```bash
# Linux - verificar que es estático
ldd dist/overlay-native-linux/overlay-native
# Debe mostrar "not a dynamic executable" o muy pocas dependencias

# Windows - verificar dependencias
dumpbin /dependents dist\overlay-native-windows\overlay-native.exe
# Debe mostrar solo DLLs del sistema (kernel32.dll, user32.dll, etc.)
```

## 📋 Checklist de Instalación

### Pre-instalación
- [ ] Sistema operativo compatible
- [ ] Rust 1.70+ instalado
- [ ] Herramientas de compilación instaladas
- [ ] Dependencias del sistema instaladas

### Compilación
- [ ] Repositorio clonado/descargado
- [ ] `cargo build` ejecuta sin errores
- [ ] `cargo test` pasa todos los tests
- [ ] `cargo run` inicia la aplicación

### Verificación
- [ ] Aplicación se conecta a Twitch
- [ ] Ventanas aparecen al recibir mensajes
- [ ] Transparencia funciona correctamente
- [ ] Texto se renderiza correctamente
- [ ] Barra de progreso se actualiza
- [ ] Ventanas se cierran automáticamente

### Distribución (Opcional)
- [ ] Compilación release exitosa
- [ ] Binario funciona sin dependencias externas
- [ ] Documentación incluida
- [ ] Paquete creado correctamente