# ARCHITECTURE.md — Explorador de Notas TUI

## Visión General

Aplicación de terminal (TUI) para explorar, planificar y editar notas en formato Markdown, inspirada en Obsidian. Construida en Go con el ecosistema Charm. Soporta navegación, ejecución de tareas y edición inline.

## Stack Tecnológico

| Biblioteca | Versión | Rol |
|---|---|---|
| bubbletea | v1.3.x | Ciclo de vida, estado, máquina de estados |
| lipgloss | v1.1.x | Layout, estilos visuales, paneles |
| glamour | v1.0.x | Renderizado de Markdown a terminal |
| bubbles | v1.0.x | Componente textinput para edición inline |

## Estructura de Archivos

```
notas/
├── main.go         # Entry point.
├── model.go        # Modelo, máquina de estados (appMode), lógica de interacción.
├── ui.go           # Layout visual, renderizado por modo, barra de estado.
├── markdown.go     # Parseo, renderizado, toggle de tareas, edición de líneas.
├── go.mod / go.sum
├── ARCHITECTURE.md
└── notes/
    ├── bienvenida.md
    ├── ideas.md
    ├── tareas.md
    └── sprint.md
```

## Layout Visual

```
┌─────────────────┬────────────────────────────────┐
│  📂 Notas       │  📄 Vista Previa               │
│                 │  (raw con cursor en ModeExec)   │  70%
│  > tareas       │  ❯ - [ ] Implementar búsqueda   │  del
│    sprint       │    - [x] Parser de front matter │  alto
│    bienvenida   │                                │
│    ideas        │                                │
├─────────────────┴────────────────────────────────┤
│  📊 Planificación / Línea de Tiempo              │  30%
│  Duración: 5 días  [██████░░░░]  60%             │  del
│                                                   │  alto
├───────────────────────────────────────────────────┤
│ [EXEC]  tareas  ✓ 2/5                            │  Status
└───────────────────────────────────────────────────┘
```

## Máquina de Estados

| Modo | Descripción | Entrada | Salida |
|---|---|---|---|
| **ModeNav** | Navegación entre notas | Por defecto | `Tab` → ModeExec |
| **ModeExec** | Cursor de línea en preview | `Tab` desde ModeNav | `Esc` → ModeNav, `e` → ModeEdit |
| **ModeEdit** | Edición inline de una línea | `e` desde ModeExec | `Enter` → ModeExec, `Esc` → ModeExec |

### Controles por Modo

**ModeNav:**
| Tecla | Acción |
|---|---|
| ↑ / k | Nota anterior |
| ↓ / j | Nota siguiente |
| Tab | Entrar a ModeExec |
| q / Ctrl+C | Salir |

**ModeExec:**
| Tecla | Acción |
|---|---|
| ↑ / k | Línea arriba |
| ↓ / j | Línea abajo |
| Espacio | Tildar/destildar tarea (`- [ ]` ↔ `- [x]`) |
| e | Editar línea activa (→ ModeEdit) |
| Tab | Volver a ModeNav |
| Esc | Volver a ModeNav |

**ModeEdit:**
| Tecla | Acción |
|---|---|
| Enter | Confirmar edición y guardar |
| Esc | Cancelar edición |
| (teclas) | Escribir en textinput |

## Responsabilidades por Archivo

### `main.go`
- Parsea argumentos (directorio de notas opcional).
- Escanea archivos `.md` al inicio.
- Crea el modelo con modo `ModeNav` (lazy loading).

### `model.go`
- Define `appMode` (`ModeNav`, `ModeExec`, `ModeEdit`).
- Struct `model` con estado completo: cursor de nota, cursor de línea, modo, textinput, rawLines, meta.
- `updateNavExec()`: lógica de navegación y ejecución (toggle tasks).
- `updateEdit()`: delega al textinput, Enter guarda línea, Esc cancela.
- Helpers: `taskCounts()`, `activeFileName()`, `modeString()`.

### `ui.go`
- Layout de 3 filas: paneles superiores (70%), planificación (30%), barra de estado (1 línea).
- `renderRawLines()`: dibuja contenido raw con highlight de línea activa y textinput embebido.
- `renderStatusBar()`: muestra modo, archivo activo y contador de tareas.
- En `ModeNav` muestra contenido Glamour renderizado; en `ModeExec`/`ModeEdit` muestra raw con cursor.

### `markdown.go`
- `parseMeta()`: parsea front matter YAML.
- `loadNoteCmd()`: carga asíncrona, devuelve contenido renderizado + rawLines + meta.
- `toggleTask()`: intercambia `- [ ]` ↔ `- [x]` en la línea indicada y guarda archivo.
- `replaceLine()`: reemplaza una línea del body y guarda archivo.
- `rebuildFile()`: reconstruye archivo con front matter + body modificado.
- `countTasks()`: cuenta tareas completadas vs total en un slice de líneas.

## Formato de Metadatos

```yaml
---
duracion: 5
progreso: 60
---

# Contenido de la nota...
```

## Estado Actual — Paso 4 (Interactividad Total)

- [x] Estructura modular creada.
- [x] Layout responsivo de 3 paneles + barra de estado.
- [x] Lista de archivos `.md` con navegación.
- [x] Vista previa con renderizado Markdown (Glamour).
- [x] Panel de planificación con barra de progreso.
- [x] Parser de front matter YAML.
- [x] Carga asíncrona con `tea.Cmd`.
- [x] **Máquina de estados**: ModeNav, ModeExec, ModeEdit.
- [x] **Cursor de línea** visual en ModeExec (highlight).
- [x] **Toggle de tareas** con Espacio (guarda en `.md` real).
- [x] **Edición inline** con bubbles/textinput (Enter guarda, Esc cancela).
- [x] **Barra de estado** con modo, archivo y contador de tareas.

## Decisiones de Diseño

- **Arranque lazy**: sin renderer ni contenido al inicio.
- **Modos separados**: j/k cambia de nota en ModeNav, de línea en ModeExec. Sin conflicto.
- **Vista dual**: ModeNav usa Glamour renderizado; ModeExec/ModeEdit usa líneas raw para permitir cursor y edición precisa.
- **Guardado directo**: toggle y edición escriben el archivo `.md` inmediatamente y recargan la nota.
- **Status bar**: altura fija de 1 línea, siempre visible, no interfiere con los paneles.

## Próximos Pasos

- [ ] **Paso 5**: Búsqueda y filtrado de notas.
- [ ] **Paso 6**: Sistema de tags y enlaces entre notas.
- [ ] **Paso 7**: Creación de nuevas notas desde la TUI.

## Convenciones

- Las notas se almacenan como `.md` en `notes/` (o directorio pasado como argumento).
- Layout responsivo al tamaño de la terminal.
- Colores compatibles con terminales de 256 colores.
- Metadatos opcionales en YAML front matter.
- Tareas en formato GitHub: `- [ ]` y `- [x]`.
