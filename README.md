# Overlay Native

Un cliente de overlay nativo para Twitch Chat que muestra mensajes de chat como ventanas flotantes semi-transparentes en el escritorio.

## 🚀 Características

- **Multiplataforma**: Soporte nativo para Linux (GTK) y Windows (WinAPI)
- **Overlay en tiempo real**: Muestra mensajes de Twitch chat como ventanas flotantes
- **Ventanas semi-transparentes**: Overlay no intrusivo con transparencia configurable
- **Barra de progreso**: Indicador visual del tiempo de vida de cada mensaje
- **Posicionamiento aleatorio**: Los mensajes aparecen en posiciones aleatorias en la pantalla
- **Gestión automática**: Las ventanas se cierran automáticamente después de 10 segundos
- **Renderizado de emotes**: Soporte para emotes de Twitch (animados y estáticos)

## 📋 Requisitos del Sistema

### Linux
- GTK 3.0+
- GDK 3.0+
- X11 (para funcionalidades específicas de ventanas)
- Rust 1.70+

### Windows
- Windows 10/11
- Visual Studio Build Tools o Visual Studio Community
- Rust 1.70+

## 🛠️ Instalación

Ver [Guía de Instalación](docs/INSTALLATION.md) para instrucciones detalladas por plataforma.

```bash
# Clonar el repositorio
git clone <repository-url>
cd overlay-native

# Compilar y ejecutar
cargo run
```

## 📖 Documentación

- [Especificaciones Linux](docs/LINUX_SPECS.md) - Detalles técnicos para la implementación GTK
- [Especificaciones Windows](docs/WINDOWS_SPECS.md) - Detalles técnicos para la implementación WinAPI
- [Arquitectura del Código](docs/ARCHITECTURE.md) - Estructura y diseño del proyecto
- [Guía de Instalación](docs/INSTALLATION.md) - Instrucciones de compilación por plataforma

## 🏗️ Arquitectura

El proyecto está estructurado con módulos específicos por plataforma:

```
src/
├── main.rs          # Punto de entrada principal
├── connection.rs    # Cliente IRC de Twitch
├── window.rs        # Implementación GTK (Linux)
├── windows.rs       # Implementación WinAPI (Windows)
└── x11.rs          # Funcionalidades específicas de X11
```

## 🎮 Uso

1. Ejecuta la aplicación con `cargo run`
2. La aplicación se conectará automáticamente al canal de Twitch configurado
3. Los mensajes de chat aparecerán como ventanas flotantes en tu pantalla
4. Cada ventana muestra:
   - Nombre de usuario (en negrita)
   - Contenido del mensaje
   - Emotes de Twitch (si están presentes)
   - Barra de progreso indicando el tiempo restante

## ⚙️ Configuración

Actualmente el canal está hardcodeado en `main.rs`. Para cambiar el canal:

```rust
client.join("tu_canal_aqui".to_owned()).unwrap();
```

## 🤝 Contribuir

1. Fork el proyecto
2. Crea una rama para tu feature (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add some AmazingFeature'`)
4. Push a la rama (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

## 📝 Licencia

Este proyecto está bajo la licencia MIT. Ver el archivo `LICENSE` para más detalles.

## 🐛 Problemas Conocidos

- En Windows, las ventanas pueden no aparecer correctamente si no se tienen los permisos adecuados
- En Linux, requiere un servidor X11 funcionando
- Los emotes pueden tardar en cargar dependiendo de la conexión a internet

## 🔧 Desarrollo

Para desarrollo local:

```bash
# Ejecutar en modo debug
cargo run

# Ejecutar tests
cargo test

# Compilar para release
cargo build --release
```

## 📊 Estado del Proyecto

- ✅ Conexión a Twitch IRC
- ✅ Renderizado de ventanas en Windows
- ✅ Renderizado de ventanas en Linux
- ✅ Barra de progreso funcional
- ✅ Gestión de memoria y limpieza
- 🔄 Carga de emotes (en progreso)
- ⏳ Configuración por archivo
- ⏳ Interfaz gráfica de configuración