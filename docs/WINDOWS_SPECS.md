# Especificaciones Técnicas - Windows (WinAPI)

## Resumen
Este documento describe la implementación Windows de Overlay Native basada en WinAPI (user32, gdi32) con ventanas superpuestas y transparencia.

## Stack
- WinAPI vía crate winapi
- User32 para creación y gestión de ventanas
- GDI para renderizado básico

## Ventanas y Ciclo de Vida
- Creación de ventanas con estilos adecuados (layered windows) para lograr transparencia.
- Estructura WindowsWindow con HWND y estado de progreso.
- Métodos principales: new, close, set_progress.
- Renderizado del contenido en función del progreso y texto.

## Emotes
- Estado: implementado parcialmente y sujeto a futuras mejoras.

## Consideraciones de Threading
- El message loop de Windows se ejecuta en el thread que creó la ventana.
- Tokio se usa para tareas asíncronas de red (twitch-irc) y gestión de timers.

## Requisitos del Sistema
- Windows 10/11 con toolchain MSVC o MinGW.

## Limitaciones Conocidas
- Comportamientos de z-order y click-through pueden depender de configuraciones del sistema.