# Especificaciones Técnicas - Linux (GTK)

## 🐧 Implementación Linux

La implementación para Linux utiliza GTK 3.0+ y GDK para crear ventanas overlay semi-transparentes que se muestran sobre otras aplicaciones.

## 📦 Dependencias

### Dependencias del Sistema
```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev libgdk-pixbuf2.0-dev libatk1.0-dev libcairo-gobject2 libpango1.0-dev libgdk-pixbuf2.0-dev libglib2.0-dev

# Fedora/RHEL
sudo dnf install gtk3-devel gdk-pixbuf2-devel atk-devel cairo-gobject-devel pango-devel glib2-devel

# Arch Linux
sudo pacman -S gtk3 gdk-pixbuf2 atk cairo pango glib2
```

### Dependencias de Rust
```toml
[dependencies]
gtk = "0.18"
gdk = "0.18"
glib = "0.18"
cairo-rs = "0.18"
pango = "0.18"
```

## 🏗️ Arquitectura de Ventanas

### Estructura Principal
```rust
pub struct LinuxWindow {
    window: gtk::Window,
    drawing_area: gtk::DrawingArea,
    progress: Arc<Mutex<f64>>,
    username: String,
    message: String,
    created_time: std::time::Instant,
}
```

### Características de Ventana

#### Configuración de Ventana
- **Tipo**: `gtk::WindowType::Popup`
- **Decoraciones**: Deshabilitadas (`set_decorated(false)`)
- **Redimensionable**: No (`set_resizable(false)`)
- **Siempre encima**: Sí (`set_keep_above(true)`)
- **Nivel de ventana**: `gdk::WindowTypeHint::Notification`

#### Transparencia y Composición
```rust
// Configuración de transparencia
window.set_app_paintable(true);
if let Some(screen) = window.screen() {
    if let Some(visual) = screen.rgba_visual() {
        window.set_visual(Some(&visual));
    }
}
```

#### Posicionamiento
- **Posición inicial**: Aleatoria en pantalla
- **Cálculo de posición**:
  ```rust
  let screen_width = gdk::Screen::default().unwrap().width();
  let screen_height = gdk::Screen::default().unwrap().height();
  let x = rand::random::<i32>() % (screen_width - window_width);
  let y = rand::random::<i32>() % (screen_height - window_height);
  ```

## 🎨 Renderizado

### Sistema de Dibujo
Utiliza Cairo para el renderizado personalizado:

```rust
fn draw_callback(drawing_area: &gtk::DrawingArea, ctx: &cairo::Context) {
    // Fondo semi-transparente
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.8);
    ctx.paint();
    
    // Texto del usuario (negrita)
    ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    // ... renderizado de texto
    
    // Barra de progreso
    ctx.set_source_rgba(0.2, 0.6, 1.0, 0.8);
    // ... renderizado de barra
}
```

### Fuentes y Texto
- **Fuente principal**: "Sans 12"
- **Fuente usuario**: "Sans Bold 12"
- **Codificación**: UTF-8 completo
- **Renderizado**: Pango para texto complejo

### Colores
- **Fondo**: `rgba(0, 0, 0, 0.8)` - Negro semi-transparente
- **Texto usuario**: `rgba(255, 255, 255, 1.0)` - Blanco sólido
- **Texto mensaje**: `rgba(220, 220, 220, 1.0)` - Gris claro
- **Barra progreso**: `rgba(51, 153, 255, 0.8)` - Azul semi-transparente
- **Fondo barra**: `rgba(100, 100, 100, 0.6)` - Gris oscuro

## ⚡ Gestión de Eventos

### Eventos de Ventana
```rust
// Evento de dibujo
drawing_area.connect_draw(|_, ctx| {
    // Lógica de renderizado
    Inhibit(false)
});

// Evento de destrucción
window.connect_delete_event(|_, _| {
    // Limpieza de recursos
    Inhibit(false)
});
```

### Temporizadores
- **Actualización de progreso**: 50ms (20 FPS)
- **Duración total**: 10 segundos
- **Auto-cierre**: Automático al completar progreso

## 🔧 Configuración del Sistema

### Requisitos del Entorno
1. **Servidor X11**: Requerido para funcionalidades de ventana
2. **Compositor**: Recomendado para transparencia suave
3. **Permisos**: No requiere permisos especiales

### Variables de Entorno
```bash
# Para debugging GTK
export GTK_DEBUG=all
export G_MESSAGES_DEBUG=all

# Para forzar backend X11
export GDK_BACKEND=x11
```

## 🐛 Problemas Conocidos y Soluciones

### Transparencia no Funciona
**Problema**: Las ventanas aparecen opacas
**Solución**:
```bash
# Verificar compositor
ps aux | grep -i compos

# Instalar compositor si es necesario
sudo apt-get install compton  # o picom
```

### Ventanas no Aparecen
**Problema**: Las ventanas se crean pero no son visibles
**Solución**:
```rust
// Asegurar que la ventana se muestre
window.show_all();
window.present();
```

### Problemas de Fuentes
**Problema**: Texto no se renderiza correctamente
**Solución**:
```bash
# Instalar fuentes básicas
sudo apt-get install fonts-dejavu-core fonts-liberation
```

## 📊 Rendimiento

### Optimizaciones
- **Doble buffer**: Habilitado por defecto en GTK
- **Invalidación selectiva**: Solo redibuja áreas necesarias
- **Gestión de memoria**: Liberación automática de recursos GTK

### Métricas Típicas
- **Tiempo de creación**: ~50-100ms
- **Uso de memoria**: ~2-5MB por ventana
- **CPU**: <1% en idle, ~5% durante animaciones

## 🔍 Debugging

### Herramientas de Debug
```bash
# GTK Inspector
GTK_DEBUG=interactive cargo run

# Logs detallados
G_MESSAGES_DEBUG=all cargo run

# Profiling con valgrind
valgrind --tool=memcheck cargo run
```

### Logs Importantes
```rust
// Logging personalizado
use log::{info, warn, error};

info!("Creando ventana Linux: {}x{}", width, height);
warn!("Compositor no disponible, transparencia limitada");
error!("Error al crear ventana: {}", e);
```

## 🚀 Compilación Optimizada

### Release Build
```bash
# Compilación optimizada
cargo build --release

# Con optimizaciones específicas
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Configuración Cargo.toml
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

## 📋 Lista de Verificación

### Pre-requisitos
- [ ] GTK 3.0+ instalado
- [ ] Dependencias de desarrollo instaladas
- [ ] Servidor X11 funcionando
- [ ] Compositor habilitado (opcional pero recomendado)

### Testing
- [ ] Ventanas aparecen correctamente
- [ ] Transparencia funciona
- [ ] Texto se renderiza bien
- [ ] Barra de progreso se actualiza
- [ ] Ventanas se cierran automáticamente
- [ ] No hay memory leaks

### Distribución
- [ ] Binario compilado estáticamente
- [ ] Dependencias documentadas
- [ ] Scripts de instalación incluidos