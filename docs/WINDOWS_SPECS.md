# Especificaciones Técnicas - Windows (WinAPI)

## 🪟 Implementación Windows

La implementación para Windows utiliza WinAPI nativo para crear ventanas overlay semi-transparentes con capacidades de layered window y transparencia por canal alfa.

## 📦 Dependencias

### Dependencias del Sistema
- Windows 10/11 (recomendado)
- Windows 8.1+ (mínimo)
- Visual Studio Build Tools 2019+ o Visual Studio Community
- Windows SDK 10.0+

### Dependencias de Rust
```toml
[dependencies]
winapi = { version = "0.3", features = [
    "winuser",
    "wingdi",
    "libloaderapi",
    "processthreadsapi",
    "memoryapi",
    "handleapi",
    "errhandlingapi"
]}
user32-sys = "0.2"
kernel32-sys = "0.2"
```

## 🏗️ Arquitectura de Ventanas

### Estructura Principal
```rust
pub struct WindowsWindow {
    hwnd: HWND,
    progress: f64,
    username: String,
    message: String,
}

#[repr(C)]
struct WindowData {
    progress: f64,
    created_time: std::time::Instant,
}
```

### Registro de Clase de Ventana
```rust
static REGISTER_CLASS: Once = Once::new();

REGISTER_CLASS.call_once(|| {
    let class_name = to_wide_string("OverlayWindowClass");
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: GetModuleHandleW(ptr::null()),
        hbrBackground: CreateSolidBrush(RGB(0, 0, 0)),
        lpszClassName: class_name.as_ptr(),
        hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
        // ... otros campos
    };
    RegisterClassW(&wc);
});
```

### Características de Ventana

#### Estilos de Ventana
- **Estilo básico**: `WS_POPUP | WS_VISIBLE`
- **Estilo extendido**: `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW`
- **Transparencia**: Habilitada via `WS_EX_LAYERED`
- **Siempre encima**: `WS_EX_TOPMOST`
- **Sin barra de tareas**: `WS_EX_TOOLWINDOW`

#### Configuración de Transparencia
```rust
// Configurar ventana layered con transparencia
SetLayeredWindowAttributes(
    hwnd,
    0,                    // Color key (no usado)
    200,                  // Alpha (0-255, 200 = ~78% opaco)
    LWA_ALPHA            // Usar canal alfa
);
```

#### Cálculo de Tamaño Dinámico
```rust
// Calcular ancho basado en longitud del texto
let base_width = 300;
let char_width = 8;
let total_text_len = username.len() + message.len() + 3; // +3 para ": "
let calculated_width = base_width + (total_text_len * char_width);
let width = calculated_width.min(800).max(200); // Límites min/max
let height = 80;
```

## 🎨 Renderizado

### Sistema de Dibujo
Utiliza GDI+ para renderizado personalizado en el procedimiento de ventana:

```rust
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            
            // Configurar fondo
            let brush = CreateSolidBrush(RGB(20, 20, 20));
            FillRect(hdc, &ps.rcPaint, brush);
            
            // Renderizar texto y barra de progreso
            render_content(hdc, hwnd);
            
            EndPaint(hwnd, &ps);
            DeleteObject(brush as *mut c_void);
            0
        }
        // ... otros mensajes
    }
}
```

### Renderizado de Texto
```rust
fn render_text(hdc: HDC, text: &str, x: i32, y: i32, bold: bool) {
    let font = CreateFontW(
        16,                           // Altura
        0,                           // Ancho (auto)
        0, 0,                        // Escapement, orientación
        if bold { FW_BOLD } else { FW_NORMAL },
        FALSE, FALSE, FALSE,         // Italic, underline, strikeout
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        DEFAULT_QUALITY,
        DEFAULT_PITCH | FF_DONTCARE,
        to_wide_string("Segoe UI").as_ptr(),
    );
    
    SelectObject(hdc, font as *mut c_void);
    SetTextColor(hdc, RGB(255, 255, 255));
    SetBkMode(hdc, TRANSPARENT);
    
    let wide_text = to_wide_string(text);
    TextOutW(hdc, x, y, wide_text.as_ptr(), wide_text.len() as i32 - 1);
    
    DeleteObject(font as *mut c_void);
}
```

### Barra de Progreso
```rust
fn render_progress_bar(hdc: HDC, rect: &RECT, progress: f64) {
    let bar_height = 4;
    let bar_y = rect.bottom - 10;
    
    // Fondo de la barra
    let bg_brush = CreateSolidBrush(RGB(60, 60, 60));
    let bg_rect = RECT {
        left: rect.left + 10,
        top: bar_y,
        right: rect.right - 10,
        bottom: bar_y + bar_height,
    };
    FillRect(hdc, &bg_rect, bg_brush);
    
    // Barra de progreso
    let progress_width = ((bg_rect.right - bg_rect.left) as f64 * progress) as i32;
    let progress_brush = CreateSolidBrush(RGB(0, 120, 215)); // Azul Windows
    let progress_rect = RECT {
        left: bg_rect.left,
        top: bg_rect.top,
        right: bg_rect.left + progress_width,
        bottom: bg_rect.bottom,
    };
    FillRect(hdc, &progress_rect, progress_brush);
    
    DeleteObject(bg_brush as *mut c_void);
    DeleteObject(progress_brush as *mut c_void);
}
```

## 💾 Gestión de Datos de Ventana

### Almacenamiento de Datos
```rust
// Almacenar datos en la ventana
let window_data = Box::new(WindowData {
    progress: 0.0,
    created_time: Instant::now(),
});
let data_ptr = Box::into_raw(window_data) as isize;
SetWindowLongPtrW(hwnd, GWLP_USERDATA, data_ptr);
```

### Recuperación de Datos
```rust
// Recuperar datos de la ventana
let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
if !data_ptr.is_null() {
    let window_data = &*data_ptr;
    let progress = window_data.progress;
    // ... usar datos
}
```

### Limpieza de Memoria
```rust
// En WM_DESTROY
WM_DESTROY => {
    let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
    if !data_ptr.is_null() {
        let _ = Box::from_raw(data_ptr); // Liberar memoria
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
    0
}
```

## ⚡ Gestión de Eventos

### Mensajes de Ventana Principales
```rust
match msg {
    WM_PAINT => {
        // Renderizado de contenido
    },
    WM_TIMER => {
        // Actualización de progreso
        update_progress(hwnd);
    },
    WM_DESTROY => {
        // Limpieza de recursos
        cleanup_window_data(hwnd);
    },
    WM_CLOSE => {
        // Cerrar ventana
        DestroyWindow(hwnd);
    },
    _ => DefWindowProcW(hwnd, msg, wparam, lparam),
}
```

### Temporizadores
```rust
// Configurar temporizador para actualizaciones
SetTimer(
    hwnd,
    1,                    // Timer ID
    50,                   // 50ms = 20 FPS
    None                  // Usar WM_TIMER
);
```

## 🔧 Configuración del Sistema

### Permisos Requeridos
- **Ejecución básica**: No requiere permisos especiales
- **Ventanas overlay**: Funciona sin UAC
- **Siempre encima**: Disponible para todas las aplicaciones

### Compatibilidad DPI
```rust
// Configurar awareness de DPI
SetProcessDPIAware();

// O para aplicaciones más modernas
SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
```

### Variables de Entorno
```cmd
REM Para debugging
set RUST_LOG=debug
set RUST_BACKTRACE=1

REM Para compilación
set RUSTFLAGS="-C target-feature=+crt-static"
```

## 🐛 Problemas Conocidos y Soluciones

### Ventanas no Aparecen
**Problema**: `CreateWindowExW` retorna NULL
**Solución**:
```rust
let hwnd = CreateWindowExW(/* parámetros */);
if hwnd.is_null() {
    let error = GetLastError();
    eprintln!("Error creando ventana: {}", error);
    // Verificar registro de clase
}
```

### Transparencia no Funciona
**Problema**: Ventana aparece opaca
**Solución**:
```rust
// Asegurar que WS_EX_LAYERED esté configurado
let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED);

// Configurar transparencia después de crear ventana
SetLayeredWindowAttributes(hwnd, 0, alpha_value, LWA_ALPHA);
```

### Memory Leaks
**Problema**: Memoria no se libera correctamente
**Solución**:
```rust
// Siempre limpiar en WM_DESTROY
// Usar RAII con Box::from_raw
// Verificar que todos los handles GDI se liberen
```

### Problemas de Fuentes
**Problema**: Texto no se renderiza o aparece corrupto
**Solución**:
```rust
// Usar fuentes del sistema
let font = CreateFontW(
    // ... parámetros ...
    to_wide_string("Segoe UI").as_ptr(), // Fuente estándar de Windows
);

// Verificar que la fuente se creó correctamente
if font.is_null() {
    // Usar fuente por defecto
    let font = GetStockObject(DEFAULT_GUI_FONT);
}
```

## 📊 Rendimiento

### Optimizaciones
- **Invalidación selectiva**: Solo redibuja cuando es necesario
- **Doble buffering**: Usar `WS_EX_COMPOSITED` si es necesario
- **Gestión eficiente de GDI**: Reutilizar objetos cuando sea posible

### Métricas Típicas
- **Tiempo de creación**: ~20-50ms
- **Uso de memoria**: ~1-3MB por ventana
- **CPU**: <1% en idle, ~3% durante animaciones
- **Handles GDI**: ~5-10 por ventana

## 🔍 Debugging

### Herramientas de Debug
```cmd
REM Spy++ para inspeccionar ventanas
"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\Common7\Tools\spyxx.exe"

REM Process Monitor para I/O
procmon.exe

REM Application Verifier para memory leaks
appverif.exe
```

### Logging Personalizado
```rust
use log::{info, warn, error};
use winapi::um::errhandlingapi::GetLastError;

fn log_windows_error(operation: &str) {
    let error_code = unsafe { GetLastError() };
    if error_code != 0 {
        error!("{} falló con código: 0x{:08X}", operation, error_code);
    }
}

// Uso
let hwnd = unsafe { CreateWindowExW(/* ... */) };
if hwnd.is_null() {
    log_windows_error("CreateWindowExW");
}
```

## 🚀 Compilación Optimizada

### Release Build
```cmd
REM Compilación estática
set RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc

REM Con optimizaciones específicas
set RUSTFLAGS="-C target-cpu=native -C target-feature=+crt-static"
cargo build --release
```

### Configuración Cargo.toml
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

## 📋 Lista de Verificación

### Pre-requisitos
- [ ] Windows 10/11 instalado
- [ ] Visual Studio Build Tools configurado
- [ ] Rust toolchain para Windows MSVC
- [ ] Windows SDK disponible

### Testing
- [ ] Ventanas aparecen en posición correcta
- [ ] Transparencia funciona correctamente
- [ ] Texto se renderiza con fuente correcta
- [ ] Barra de progreso se actualiza suavemente
- [ ] Ventanas se cierran automáticamente
- [ ] No hay memory leaks (verificar con Application Verifier)
- [ ] Funciona en diferentes resoluciones DPI

### Distribución
- [ ] Binario compilado estáticamente
- [ ] No requiere runtime adicional
- [ ] Funciona sin instalación
- [ ] Compatible con Windows Defender

## 🔒 Consideraciones de Seguridad

### Mitigaciones Implementadas
- **ASLR**: Habilitado por defecto en Rust
- **DEP**: Habilitado automáticamente
- **Stack cookies**: Incluidos en release builds
- **Control Flow Guard**: Disponible con flags adicionales

### Configuración Adicional
```toml
# En Cargo.toml para máxima seguridad
[profile.release]
overflow-checks = true
```

```cmd
REM Flags adicionales de seguridad
set RUSTFLAGS="-C target-feature=+crt-static -C control-flow-guard"
```