# ARCHITECTURE.md — Explorador de Notas TUI

## Visión General

Aplicación de terminal (TUI) para explorar y planificar notas en formato Markdown, inspirada en Obsidian. Construida en Go con el ecosistema Charm.

## Stack Tecnológico

| Biblioteca | Versión | Rol |
|---|---|---|
| bubbletea | v1.3.x | Ciclo de vida, estado, manejo de eventos |
| lipgloss | v1.1.x | Layout, estilos visuales, paneles |
| glamour | v1.0.x | Renderizado de Markdown a terminal |

## Estructura de Archivos

```
notas/
├── main.go         # Entry point. Inicializa el programa Bubble Tea.
├── model.go        # Modelo principal (estado). Implementa tea.Model.
├── ui.go           # Renderizado visual. Layout con Lip Gloss.
├── markdown.go     # Lectura y escaneo de archivos .md locales.
├── go.mod / go.sum # Dependencias del módulo.
├── ARCHITECTURE.md # Este archivo. Documentación viva del proyecto.
└── notes/          # Carpeta por defecto con notas .md de ejemplo.
    ├── bienvenida.md
    ├── ideas.md
    └── tareas.md
```

## Responsabilidades por Archivo

### `main.go`
- Parsea argumentos (directorio de notas opcional).
- Escanea archivos `.md` al inicio.
- Inicializa y ejecuta `tea.Program` con pantalla alternativa (`AltScreen`).

### `model.go`
- Define el struct `model` con todo el estado de la aplicación.
- Implementa `Init()`, `Update()`, `View()` de `tea.Model`.
- Maneja eventos de teclado (navegación ↑↓/jk, salir q/Ctrl+C).
- Maneja `tea.WindowSizeMsg` para layout responsivo.

### `ui.go`
- Renderiza el layout de dos paneles con `lipgloss.JoinHorizontal`.
- Panel izquierdo (1/3): lista de archivos con cursor y scroll.
- Panel derecho (2/3): vista previa del contenido.
- Define estilos visuales (colores, bordes, tipografía).

### `markdown.go`
- `scanNotes(dir)`: lee el directorio y devuelve archivos `.md`.
- `readNote(dir, filename)`: lee el contenido de un archivo.
- Preparado para integrar Glamour para renderizado enriquecido.

## Estado Actual — Paso 1

- [x] Estructura modular creada.
- [x] Layout responsivo de dos paneles con Lip Gloss.
- [x] Lista de archivos `.md` con navegación (↑↓ / j/k).
- [x] Vista previa del archivo seleccionado (texto plano).
- [x] Scroll automático en la lista de archivos.
- [x] Soporte para directorio personalizado vía argumento.

## Próximos Pasos

- [ ] **Paso 2**: Integrar Glamour para renderizado Markdown enriquecido.
- [ ] **Paso 3**: Creación y edición de notas desde la TUI.
- [ ] **Paso 4**: Búsqueda y filtrado de notas.
- [ ] **Paso 5**: Sistema de tags y enlaces entre notas.

## Controles

| Tecla | Acción |
|---|---|
| ↑ / k | Mover cursor arriba |
| ↓ / j | Mover cursor abajo |
| q / Ctrl+C | Salir |

## Convenciones

- Las notas se almacenan como archivos `.md` en la carpeta `notes/` (o la pasada como argumento).
- El layout se ajusta automáticamente al tamaño de la terminal.
- Colores compatibles con terminales de 256 colores.
