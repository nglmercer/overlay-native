# Especificaciones Técnicas - Linux (GTK/X11)

## Resumen
Este documento detalla la implementación Linux de Overlay Native, basada en GTK 3, GDK y utilidades X11.

## Stack
- GTK 3, GDK, Pango, GLib
- gdkx11 para integración con X11
- x11rb para manipulación de propiedades X11

## Ventanas y Ciclo de Vida
- Ventanas GTK con atributos de transparencia y sin decoración (según configuración del código).
- Etiquetas GTK para username y mensaje.
- Barra de progreso implementada con widgets/propiedades actualizadas periódicamente.
- Cierre automático ~10s tras creación (ver timers en el código).

## Integración X11
- x11.rs define X11BackendConnection y AtomCollection para establecer propiedades X11 específicas en ventanas.
- Propiedades comunes: tipo de ventana, comportamiento de stacking, etc.

## Emotes
- Estado: implementado parcialmente.
- Descarga de recursos con reqwest; pendiente completar caché/animación.

## Consideraciones de Threading
- GTK requiere operar en el thread principal para la mayoría de interacciones UI.
- Tokio maneja tareas asíncronas para la red (twitch-irc) y timers.

## Requisitos del Sistema
- X11 en ejecución (Xorg o XWayland en sesiones Wayland).
- Paquetes de desarrollo de GTK 3 y pkg-config presentes al compilar.

## Limitaciones Conocidas
- En sesiones Wayland puras, algunas features X11 pueden no estar disponibles.
- El overlay puede estar sujeto a políticas del compositor (siempre encima, input passthrough, etc.).