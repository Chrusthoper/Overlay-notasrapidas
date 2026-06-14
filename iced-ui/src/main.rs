use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, text_input, Space};
use iced::{Background, Border, Color, Element, Event, Length, Padding, Point, Task, Theme};
use iced_layershell::build_pattern::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;
use serde::{Deserialize, Serialize};

use overlay_core::{append_to_note, get_recent_notes, load_config_embedded, open_tui, open_tui_with_file, read_note, replace_line};

const CONFIG_TOML: &str = include_str!("../../src-tauri/config.toml");

const BG: Color = Color { r: 18.0 / 255.0, g: 18.0 / 255.0, b: 18.0 / 255.0, a: 0.92 };
const FG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.85 };
const FG_DIM: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.45 };
const MUTED: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const GREEN: Color = Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 1.0 };
const BORDER_CLR: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 };
const BLUE: Color = Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.8 };
const RED: Color = Color { r: 243.0 / 255.0, g: 139.0 / 255.0, b: 168.0 / 255.0, a: 1.0 };
const PURPLE: Color = Color { r: 137.0 / 255.0, g: 180.0 / 250.0, b: 250.0 / 255.0, a: 0.7 };
const EXPANDED_BG: Color = Color { r: 15.0 / 255.0, g: 15.0 / 255.0, b: 25.0 / 255.0, a: 0.97 };
const TASK_BORDER: Color = Color { r: 137.0 / 255.0, g: 180.0 / 250.0, b: 250.0 / 255.0, a: 0.4 };

const RESIZE_ZONE: f32 = 12.0;
const RESIZE_CORNER: f32 = 20.0;
const MIN_W: u32 = 300;
const MAX_W: u32 = 800;
const MIN_H: u32 = 200;
const MAX_H: u32 = 600;

#[derive(Debug, Clone, Copy, PartialEq)]
enum OverlayMode {
    Normal,
    CreatingNote,
}

#[derive(Clone, Debug, PartialEq)]
enum ResizeEdge {
    N, S, E, W, NE, NW, SE, SW,
}

#[derive(Debug, Clone)]
struct CtxMenu {
    note_name: String,
    confirming_delete: bool,
}

#[derive(Debug, Clone)]
struct TaskItem {
    line_index: usize,
    text: String,
    done: bool,
}

#[derive(Debug, Clone)]
struct ContentLine {
    line_index: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct EditingLine {
    line_index: usize,
    value: String,
    is_task: bool,
    was_done: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct PanelState {
    #[serde(default)]
    pinned: Vec<String>,
    #[serde(default)]
    hidden: Vec<String>,
    #[serde(default)]
    window_w: u32,
    #[serde(default)]
    window_h: u32,
}

fn panel_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("overlay")
        .join("panel.json")
}

fn load_panel() -> PanelState {
    let path = panel_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_panel(pinned: &[String], hidden: &[String], window_size: (u32, u32)) {
    let path = panel_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let state = PanelState {
        pinned: pinned.to_vec(),
        hidden: hidden.to_vec(),
        window_w: window_size.0,
        window_h: window_size.1,
    };
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&state).unwrap_or_default());
}

fn parse_tasks(content: &str) -> Vec<TaskItem> {
    let mut tasks = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("- [X] ")) {
            tasks.push(TaskItem { line_index: i, text: rest.to_string(), done: true });
        } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            tasks.push(TaskItem { line_index: i, text: rest.to_string(), done: false });
        }
    }
    tasks
}

fn parse_content_lines(content: &str) -> Vec<ContentLine> {
    let lines: Vec<&str> = content.lines().collect();
    let fm_end = find_front_matter_end(content);
    let start = if fm_end > 0 { fm_end } else { 0 };
    let mut result = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i < start {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") || trimmed.starts_with("- [ ] ") {
            continue;
        }
        result.push(ContentLine { line_index: i, text: trimmed.to_string() });
    }
    result
}

fn find_front_matter_end(content: &str) -> usize {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") { return 0; }
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let closing_line_end = end + 4;
        let bytes = trimmed.as_bytes();
        let mut idx = closing_line_end;
        while idx < bytes.len() && (bytes[idx] == b'\n' || bytes[idx] == b'\r') {
            idx += 1;
        }
        let prefix_len = content.len() - trimmed.len();
        let line_count = content[..prefix_len + idx].lines().count();
        line_count
    } else {
        0
    }
}

fn toggle_task_in_file(notes_dir: &std::path::PathBuf, filename: &str, line_index: usize) -> Result<(), String> {
    let path = notes_dir.join(filename);
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    if line_index < lines.len() {
        let line = &mut lines[line_index];
        let trimmed = line.trim();
        if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
            *line = line.replacen("- [x]", "- [ ]", 1).replacen("- [X]", "- [ ]", 1);
        } else if trimmed.starts_with("- [ ] ") {
            *line = line.replacen("- [ ]", "- [x]", 1);
        }
    }
    let result = lines.join("\n");
    std::fs::write(&path, result).map_err(|e| e.to_string())
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    InputSubmitted,
    NoteClicked(usize),
    NoteCtxClicked(usize),
    TaskToggled(usize),
    LineEditStarted(usize, bool, bool),
    LineEditChanged(String),
    LineEditSubmitted,
    LineEditCancelled,
    TuiClicked,
    TuiForNote(String),
    CloseClicked,
    NewNoteClicked,
    CreateNote,
    CancelCreate,
    EscapePressed,
    CtxViewTui(String),
    CtxPin(String),
    CtxHide(String),
    CtxDelete(String),
    CtxConfirmDelete(String),
    CtxCancel,
    DragStarted,
    DragMoved(f32, f32),
    DragEnded,
    DragHover(bool),
    ResizeStarted(ResizeEdge),
    ResizeEnded,
}

struct Overlay {
    mode: OverlayMode,
    input_value: String,
    notes: Vec<overlay_core::NoteInfo>,
    notes_dir: std::path::PathBuf,
    status_msg: String,
    selected_file: String,
    session_file: String,
    pinned: Vec<String>,
    hidden: Vec<String>,
    ctx_menu: Option<CtxMenu>,
    expanded_note: Option<String>,
    expanded_tasks: Vec<TaskItem>,
    expanded_content_lines: Vec<ContentLine>,
    editing_line: Option<EditingLine>,
    dragging: bool,
    drag_hovered: bool,
    window_size: (u32, u32),
    resizing: Option<ResizeEdge>,
    resize_origin: Point,
    resize_size_start: (u32, u32),
    resize_margin_start: (i32, i32),
    margin: (i32, i32, i32, i32),
    last_cursor: Point,
}

impl Overlay {
    fn new() -> Self {
        let config = load_config_embedded(CONFIG_TOML);
        let notes_dir = overlay_core::resolve_notes_path(&config);
        let notes = get_recent_notes(&notes_dir);
        let now = chrono::Local::now();
        let session_file = format!("inbox-{}.md", now.format("%Y-%m-%d-%Hh%M"));
        let panel = load_panel();
        Overlay {
            mode: OverlayMode::Normal,
            input_value: String::new(),
            notes,
            notes_dir,
            status_msg: String::new(),
            selected_file: session_file.clone(),
            session_file,
            pinned: panel.pinned,
            hidden: panel.hidden,
            ctx_menu: None,
            expanded_note: None,
            expanded_tasks: Vec::new(),
            expanded_content_lines: Vec::new(),
            editing_line: None,
            dragging: false,
            drag_hovered: false,
            window_size: (
                if panel.window_w > 0 { panel.window_w.clamp(MIN_W, MAX_W) } else { 428 },
                if panel.window_h > 0 { panel.window_h.clamp(MIN_H, MAX_H) } else { 280 },
            ),
            resizing: None,
            resize_origin: Point::default(),
            resize_size_start: (0, 0),
            resize_margin_start: (0, 0),
            margin: (0, 0, 0, 0),
            last_cursor: Point::default(),
        }
    }

    fn sorted_visible_notes(&self) -> Vec<&overlay_core::NoteInfo> {
        let mut vis: Vec<&overlay_core::NoteInfo> = self.notes
            .iter()
            .filter(|n| !self.hidden.contains(&n.name))
            .collect();
        vis.sort_by(|a, b| {
            let a_pinned = self.pinned.contains(&a.name);
            let b_pinned = self.pinned.contains(&b.name);
            let a_session = format!("{}.md", a.name) == self.session_file;
            let b_session = format!("{}.md", b.name) == self.session_file;
            match (a_pinned, b_pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_session, b_session) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.modified.cmp(&a.modified),
                },
            }
        });
        vis
    }

    fn calc_window_height(&self) -> u32 {
        let note_count = self.sorted_visible_notes().len() as u32;
        let list_height = note_count * 36;
        let panel_height = if self.expanded_note.is_some() {
            let content_lines: u32 = self.expanded_content_lines.iter()
                .map(|l| if l.text.len() > 50 { 36 } else { 18 })
                .sum();
            let task_lines: u32 = self.expanded_tasks.len() as u32 * 24;
            let base: u32 = 30 + 20;
            (base + content_lines + task_lines).clamp(200, 400)
        } else {
            0
        };
        let total = 26 + 40 + 40 + list_height + panel_height + 32;
        let height = total.clamp(248, 520);
        eprintln!("[HEIGHT] calc={} notes={} panel={}", height, note_count, panel_height);
        height
    }

    fn refresh_notes(&mut self) {
        self.notes = get_recent_notes(&self.notes_dir);
    }

    fn reload_expanded_tasks(&mut self) {
        if let Some(ref name) = self.expanded_note {
            let filename = format!("{}.md", name);
            if let Ok(content) = read_note(&self.notes_dir, &filename) {
                self.expanded_tasks = parse_tasks(&content);
                self.expanded_content_lines = parse_content_lines(&content);
            }
        }
    }

    fn expand_note(&mut self, name: &str) {
        let filename = format!("{}.md", name);
        self.selected_file = filename.clone();
        self.expanded_note = Some(name.to_string());
        if let Ok(content) = read_note(&self.notes_dir, &filename) {
            self.expanded_tasks = parse_tasks(&content);
            self.expanded_content_lines = parse_content_lines(&content);
        } else {
            self.expanded_tasks = Vec::new();
            self.expanded_content_lines = Vec::new();
        }
    }

    fn save_editing_line(&mut self) {
        if let Some(ref el) = self.editing_line.take() {
            if let Some(ref name) = self.expanded_note {
                let filename = format!("{}.md", name);
                let new_line = if el.is_task {
                    if el.was_done {
                        format!("- [x] {}", el.value.trim())
                    } else {
                        format!("- [ ] {}", el.value.trim())
                    }
                } else {
                    el.value.clone()
                };
                let _ = replace_line(&self.notes_dir, &filename, el.line_index, &new_line);
                self.reload_expanded_tasks();
                self.refresh_notes();
            }
        }
    }

    fn collapse_note(&mut self) {
        self.expanded_note = None;
        self.expanded_tasks = Vec::new();
        self.expanded_content_lines = Vec::new();
        self.editing_line = None;
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
                Task::none()
            }
            Message::InputSubmitted => {
                if self.mode == OverlayMode::CreatingNote {
                    return self.handle_create_note();
                }
                let trimmed = self.input_value.trim().to_string();
                if !trimmed.is_empty() {
                    match append_to_note(&self.notes_dir, &self.selected_file, &trimmed) {
                        Ok(_) => {
                            self.status_msg = "✓".to_string();
                            self.input_value.clear();
                            self.refresh_notes();
                            if self.expanded_note.is_some() {
                                self.reload_expanded_tasks();
                            }
                        }
                        Err(e) => {
                            self.status_msg = format!("✗ {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::NoteClicked(idx) => {
                let notes = self.sorted_visible_notes();
                if idx < notes.len() {
                    let name = notes[idx].name.clone();
                    if self.expanded_note.as_deref() == Some(&name) {
                        self.collapse_note();
                    } else {
                        self.expand_note(&name);
                    }
                }
                let new_h = self.calc_window_height();
                Task::done(Message::SizeChange((428, new_h)))
            }
            Message::NoteCtxClicked(idx) => {
                let notes = self.sorted_visible_notes();
                if idx < notes.len() {
                    self.ctx_menu = Some(CtxMenu {
                        note_name: notes[idx].name.clone(),
                        confirming_delete: false,
                    });
                }
                Task::none()
            }
            Message::TaskToggled(line_index) => {
                if let Some(ref name) = self.expanded_note {
                    let filename = format!("{}.md", name);
                    let _ = toggle_task_in_file(&self.notes_dir, &filename, line_index);
                    self.reload_expanded_tasks();
                    self.refresh_notes();
                }
                Task::none()
            }
            Message::LineEditStarted(line_index, is_task, was_done) => {
                if let Some(ref el) = self.editing_line {
                    if el.line_index != line_index {
                        self.save_editing_line();
                    }
                }
                let value = if let Some(ref name) = self.expanded_note {
                    let filename = format!("{}.md", name);
                    read_note(&self.notes_dir, &filename)
                        .ok()
                        .and_then(|c| c.lines().nth(line_index).map(|l| l.to_string()))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                self.editing_line = Some(EditingLine { line_index, value, is_task, was_done });
                Task::none()
            }
            Message::LineEditChanged(val) => {
                if let Some(ref mut el) = self.editing_line {
                    el.value = val;
                }
                Task::none()
            }
            Message::LineEditSubmitted => {
                self.save_editing_line();
                Task::none()
            }
            Message::LineEditCancelled => {
                self.editing_line = None;
                Task::none()
            }
            Message::TuiClicked => {
                let _ = open_tui();
                Task::none()
            }
            Message::CloseClicked => {
                std::process::exit(0);
            }
            Message::TuiForNote(name) => {
                let filename = format!("{}.md", name);
                let _ = open_tui_with_file(&filename);
                Task::none()
            }
            Message::NewNoteClicked => {
                self.mode = OverlayMode::CreatingNote;
                self.input_value.clear();
                Task::none()
            }
            Message::CreateNote => self.handle_create_note(),
            Message::CancelCreate => {
                self.mode = OverlayMode::Normal;
                self.input_value.clear();
                Task::none()
            }
            Message::EscapePressed => {
                if self.editing_line.is_some() {
                    self.editing_line = None;
                    Task::none()
                } else if self.ctx_menu.is_some() {
                    self.ctx_menu = None;
                    Task::none()
                } else if self.expanded_note.is_some() {
                    self.collapse_note();
                    let new_h = self.calc_window_height();
                    Task::done(Message::SizeChange((428, new_h)))
                } else if self.mode == OverlayMode::CreatingNote {
                    self.mode = OverlayMode::Normal;
                    self.input_value.clear();
                    Task::none()
                } else {
                    self.input_value.clear();
                    self.status_msg.clear();
                    self.selected_file = self.session_file.clone();
                    Task::none()
                }
            }
            Message::CtxViewTui(name) => {
                let filename = format!("{}.md", name);
                let _ = open_tui_with_file(&filename);
                self.ctx_menu = None;
                Task::none()
            }
            Message::CtxPin(name) => {
                if self.pinned.contains(&name) {
                    self.pinned.retain(|n| n != &name);
                } else {
                    self.pinned.push(name);
                }
                save_panel(&self.pinned, &self.hidden, self.window_size);
                self.ctx_menu = None;
                Task::none()
            }
            Message::CtxHide(name) => {
                self.hidden.push(name);
                save_panel(&self.pinned, &self.hidden, self.window_size);
                self.ctx_menu = None;
                Task::none()
            }
            Message::CtxDelete(name) => {
                self.ctx_menu = Some(CtxMenu {
                    note_name: name,
                    confirming_delete: true,
                });
                Task::none()
            }
            Message::CtxConfirmDelete(name) => {
                let filename = format!("{}.md", name);
                let path = self.notes_dir.join(&filename);
                let _ = std::fs::remove_file(&path);
                if self.selected_file == filename {
                    self.selected_file = self.session_file.clone();
                }
                if self.expanded_note.as_deref() == Some(&name) {
                    self.collapse_note();
                }
                self.pinned.retain(|n| n != &name);
                self.hidden.retain(|n| n != &name);
                save_panel(&self.pinned, &self.hidden, self.window_size);
                self.refresh_notes();
                self.ctx_menu = None;
                Task::none()
            }
            Message::CtxCancel => {
                self.ctx_menu = None;
                Task::none()
            }
            Message::DragStarted => {
                self.dragging = true;
                Task::none()
            }
            Message::DragMoved(x, y) => {
                if let Some(ref edge) = self.resizing {
                    let dx = (x - self.resize_origin.x) as i32;
                    let dy = (y - self.resize_origin.y) as i32;
                    let (sw, sh) = self.resize_size_start;
                    let (sm_t, sm_r) = self.resize_margin_start;
                    let (mut new_w, mut new_h) = (sw as i32, sh as i32);
                    let mut mt = sm_t;
                    let mut mr = sm_r;

                    let north = *edge == ResizeEdge::N || *edge == ResizeEdge::NE || *edge == ResizeEdge::NW;
                    let south = *edge == ResizeEdge::S || *edge == ResizeEdge::SE || *edge == ResizeEdge::SW;
                    let east = *edge == ResizeEdge::E || *edge == ResizeEdge::NE || *edge == ResizeEdge::SE;
                    let west = *edge == ResizeEdge::W || *edge == ResizeEdge::NW || *edge == ResizeEdge::SW;

                    // Anchor::Top|Right — right edge is fixed
                    if north {
                        new_h = (sh as i32 - dy).clamp(MIN_H as i32, MAX_H as i32);
                        mt = sm_t + (sh as i32 - new_h);
                    }
                    if south {
                        new_h = (sh as i32 + dy).clamp(MIN_H as i32, MAX_H as i32);
                    }
                    if east {
                        // Right edge fixed — increasing width pushes left edge left via margin
                        new_w = (sw as i32 + dx).clamp(MIN_W as i32, MAX_W as i32);
                        mr = sm_r + (new_w - sw as i32);
                    }
                    if west {
                        // Left edge moves — width shrinks as left edge moves right
                        new_w = (sw as i32 - dx).clamp(MIN_W as i32, MAX_W as i32);
                        mr = sm_r + (new_w - sw as i32);
                    }

                    self.window_size = (new_w as u32, new_h as u32);
                    self.margin = (mt.max(0), mr.max(0), 0, 0);
                    eprintln!("[RESIZE] edge={:?} dx={} dy={} new_w={} new_h={} margin={:?}", edge, dx, dy, new_w, new_h, self.margin);
                    self.last_cursor = Point { x, y };
                    Task::batch(vec![
                        Task::done(Message::SizeChange(self.window_size)),
                        Task::done(Message::MarginChange(self.margin)),
                    ])
                } else if self.dragging {
                    let dx = (x - self.last_cursor.x) as i32;
                    let dy = (y - self.last_cursor.y) as i32;
                    self.margin = (
                        (self.margin.0 + dy).max(0),
                        (self.margin.1 - dx).max(0),
                        0,
                        0,
                    );
                    self.last_cursor = Point { x, y };
                    Task::done(Message::MarginChange(self.margin))
                } else {
                    self.last_cursor = Point { x, y };
                    Task::none()
                }
            }
            Message::DragEnded => {
                if self.resizing.is_some() {
                    save_panel(&self.pinned, &self.hidden, self.window_size);
                }
                self.dragging = false;
                self.resizing = None;
                Task::none()
            }
            Message::DragHover(hovered) => {
                self.drag_hovered = hovered;
                Task::none()
            }
            Message::ResizeStarted(edge) => {
                self.resizing = Some(edge);
                self.resize_origin = self.last_cursor;
                self.resize_size_start = self.window_size;
                self.resize_margin_start = (self.margin.0, self.margin.1);
                Task::none()
            }
            Message::ResizeEnded => {
                if self.resizing.is_some() {
                    save_panel(&self.pinned, &self.hidden, self.window_size);
                }
                self.resizing = None;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn handle_create_note(&mut self) -> Task<Message> {
        let titulo = self.input_value
            .trim()
            .to_lowercase()
            .replace(" ", "-")
            .replace("/", "-");
        if titulo.is_empty() {
            self.mode = OverlayMode::Normal;
            return Task::none();
        }
        let filename = format!("{}.md", titulo);
        let path = self.notes_dir.join(&filename);
        if !path.exists() {
            let _ = std::fs::write(&path, "");
        }
        self.selected_file = filename;
        self.mode = OverlayMode::Normal;
        self.input_value.clear();
        self.refresh_notes();
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        use iced::keyboard::{Event as KbEvent, Key, key::Named};
        use iced::mouse::{self, Event as MouseEvent};

        iced::event::listen_raw(|event, _status, _id| match event {
            Event::Keyboard(KbEvent::KeyPressed {
                key: Key::Named(Named::Escape),
                ..
            }) => Some(Message::EscapePressed),

            Event::Mouse(MouseEvent::CursorMoved { position }) => {
                Some(Message::DragMoved(position.x, position.y))
            }

            Event::Mouse(MouseEvent::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::DragEnded)
            }

            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let close_btn = button(text("✕").color(MUTED).size(16))
            .on_press(Message::CloseClicked)
            .style(close_x_style)
            .padding(Padding { top: 6.0, right: 8.0, bottom: 6.0, left: 8.0 });

        let drag_alpha = if self.dragging {
            0.55
        } else if self.drag_hovered {
            0.35
        } else {
            0.18
        };
        let drag_bar = mouse_area(
            container(
                row![
                    Space::new().width(Length::Fill).height(Length::Shrink),
                    container(Space::new().width(28.0).height(3.0))
                        .style(move |_: &Theme| container::Style {
                            background: Some(Background::Color(
                                Color { r: 1.0, g: 1.0, b: 1.0, a: drag_alpha }
                            )),
                            border: Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    Space::new().width(Length::Fill).height(Length::Shrink),
                ]
                .align_y(iced::Alignment::Center)
            )
            .height(Length::Fixed(20.0))
            .center_y(Length::Fixed(20.0))
            .width(Length::Fill)
        )
        .on_press(Message::DragStarted)
        .on_release(Message::DragEnded)
        .on_enter(Message::DragHover(true))
        .on_exit(Message::DragHover(false));

        let top_bar = row![]
            .push(drag_bar)
            .push(Space::new().width(Length::Fill))
            .push(close_btn)
            .width(Length::Fill)
            .height(Length::Fixed(26.0))
            .padding(Padding { top: 2.0, right: 8.0, bottom: 0.0, left: 8.0 });

        let mut content = column![]
            .push(top_bar)
            .push(self.view_input())
            .push(self.view_actions())
            .push(self.view_note_list())
            .push(self.view_hints())
            .spacing(0)
            .padding(Padding::new(4.0))
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(_) = &self.ctx_menu {
            content = content.push(Space::new().height(Length::Fixed(4.0)));
            content = content.push(self.view_ctx_menu());
        }

        let main_content = container(content)
            .style(panel_bg_style)
            .width(Length::Fill)
            .height(Length::Fill);

        // Resize zones overlaid on edges/corners
        let z_n  = container(resize_zone(ResizeEdge::N)).width(Length::Fill).height(Length::Fixed(RESIZE_ZONE));
        let z_s  = container(resize_zone(ResizeEdge::S)).width(Length::Fill).height(Length::Fixed(RESIZE_ZONE));
        let z_e  = container(resize_zone(ResizeEdge::E)).width(Length::Fixed(RESIZE_ZONE)).height(Length::Fill);
        let z_w  = container(resize_zone(ResizeEdge::W)).width(Length::Fixed(RESIZE_ZONE)).height(Length::Fill);
        let z_ne = container(resize_zone(ResizeEdge::NE)).width(Length::Fixed(RESIZE_CORNER)).height(Length::Fixed(RESIZE_CORNER));
        let z_nw = container(resize_zone(ResizeEdge::NW)).width(Length::Fixed(RESIZE_CORNER)).height(Length::Fixed(RESIZE_CORNER));
        let z_se = container(resize_zone(ResizeEdge::SE)).width(Length::Fixed(RESIZE_CORNER)).height(Length::Fixed(RESIZE_CORNER));
        let z_sw = container(resize_zone(ResizeEdge::SW)).width(Length::Fixed(RESIZE_CORNER)).height(Length::Fixed(RESIZE_CORNER));

        // Layout: top row (NW, N, NE), middle (W, content, E), bottom (SW, S, SE)
        let top_resize = row![z_nw, z_n, z_ne]
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_y(iced::Alignment::Center);
        let mid_resize = row![z_w, Space::new().width(Length::Fill).height(Length::Fill), z_e]
            .width(Length::Fill)
            .height(Length::Fill);
        let bot_resize = row![z_sw, z_s, z_se]
            .width(Length::Fill)
            .height(Length::Shrink)
            .align_y(iced::Alignment::Center);

        let resize_overlay = column![top_resize, mid_resize, bot_resize]
            .width(Length::Fill)
            .height(Length::Fill);

        stack![main_content, resize_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_input(&self) -> Element<'_, Message> {
        let placeholder = match self.mode {
            OverlayMode::CreatingNote => "Título de la nota...",
            OverlayMode::Normal => "Escribe y presiona Enter...",
        };

        let prompt = text("❯").color(GREEN).size(16);
        let input = text_input(placeholder, &self.input_value)
            .on_input(Message::InputChanged)
            .on_submit(Message::InputSubmitted)
            .style(text_input_style)
            .width(Length::Fill);

        let mut input_row = row![].push(prompt).push(input)
            .spacing(8)
            .align_y(iced::Alignment::Center);

        if !self.status_msg.is_empty() {
            input_row = input_row.push(text(&self.status_msg).color(GREEN).size(12));
        }
        if self.mode == OverlayMode::Normal {
            if self.selected_file != self.session_file {
                let label = self.selected_file.replace(".md", "");
                input_row = input_row.push(text(format!("→ {}", label)).color(BLUE).size(11));
            } else {
                let label = self.session_file.replace(".md", "");
                input_row = input_row.push(text(label).color(MUTED).size(10));
            }
        }

        container(input_row)
            .padding(Padding::new(8.0))
            .style(compact_input_style)
            .width(Length::Fill)
            .height(Length::Fixed(40.0))
            .into()
    }

    fn view_actions(&self) -> Element<'_, Message> {
        let row = match self.mode {
            OverlayMode::Normal => {
                row![]
                    .push(
                        button(text("+ Nueva nota").color(FG).size(11))
                            .on_press(Message::NewNoteClicked)
                            .style(action_button_style)
                            .padding(Padding::new(6.0))
                    )
                    .push(Space::new().width(Length::Fill))
                    .push(
                        button(text("🗺 Canvas").color(GREEN).size(11))
                            .on_press(Message::TuiClicked)
                            .style(action_button_style)
                            .padding(Padding::new(6.0))
                    )
            }
            OverlayMode::CreatingNote => {
                row![]
                    .push(
                        button(text("✓ Crear").color(GREEN).size(11))
                            .on_press(Message::CreateNote)
                            .style(action_button_style)
                            .padding(Padding::new(6.0))
                    )
                    .push(Space::new().width(Length::Fill))
                    .push(
                        button(text("✕ Cancelar").color(MUTED).size(11))
                            .on_press(Message::CancelCreate)
                            .style(action_button_style)
                            .padding(Padding::new(6.0))
                    )
            }
        };

        container(row.width(Length::Fill).align_y(iced::Alignment::Center))
            .padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 8.0 })
            .height(Length::Fixed(40.0))
            .width(Length::Fill)
            .into()
    }

    fn view_note_list(&self) -> Element<'_, Message> {
        let notes = self.sorted_visible_notes();
        let mut col: iced::widget::Column<'_, Message, Theme> = column![].spacing(1);

        for (idx, note) in notes.iter().enumerate() {
            let filename = format!("{}.md", note.name);
            let is_selected = filename == self.selected_file;
            let is_session = filename == self.session_file;
            let is_pinned = self.pinned.contains(&note.name);
            let is_expanded = self.expanded_note.as_deref() == Some(&note.name);

            let dot_color = if is_expanded {
                GREEN
            } else if is_session {
                BLUE
            } else {
                MUTED
            };

            let name = format_note_name(&note.name);
            let pin_label = if is_pinned { "📌 " } else { "" };
            let session_label = if is_session { " sesión" } else { "" };

            let sub_text = if note.task_count > 0 {
                format!("✓ {}/{}", note.tasks_done, note.task_count)
            } else {
                relative_time(note.modified)
            };

            let name_row = row![]
                .push(text(format!("{}{}", pin_label, name)).color(FG).size(12))
                .push(Space::new().width(Length::Fill))
                .push(text(sub_text).color(MUTED).size(10))
                .width(Length::Fill);

            let detail = if !session_label.is_empty() {
                row![].push(text(session_label).color(BLUE).size(9))
            } else {
                row![]
            };

            let cell = button(
                column![].push(name_row).push(detail)
                    .spacing(2)
            )
            .on_press(Message::NoteClicked(idx))
            .style(if is_expanded {
                expanded_note_style
            } else if is_selected {
                selected_note_style
            } else {
                note_row_style
            })
            .width(Length::Fill)
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 });

            let ctx_btn = button(text("···").size(10))
                .on_press(Message::NoteCtxClicked(idx))
                .style(dots_button_style)
                .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 });

            let row_el = row![]
                .push(text("●").color(dot_color).size(8))
                .push(cell)
                .push(ctx_btn)
                .spacing(4)
                .align_y(iced::Alignment::Center);

            let row_with_ctx = mouse_area(row_el)
                .on_right_press(Message::NoteCtxClicked(idx));

            col = col.push(row_with_ctx);

            if is_expanded {
                col = col.push(self.view_expanded_panel());
            }
        }

        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_expanded_panel(&self) -> Element<'_, Message> {
        let note_name = match &self.expanded_note {
            Some(n) => n.clone(),
            None => return Space::new().into(),
        };

        let mut panel_col: iced::widget::Column<'_, Message, Theme> = column![].spacing(4);

        // Section A — content lines
        if !self.expanded_content_lines.is_empty() {
            let total = self.expanded_content_lines.len();
            let visible: Vec<&ContentLine> = self.expanded_content_lines.iter().take(4).collect();
            let has_more = total > 4;

            let mut content_col: iced::widget::Column<'_, Message, Theme> = column![].spacing(2);
            for cl in &visible {
                let is_editing = self.editing_line.as_ref()
                    .map(|e| e.line_index == cl.line_index)
                    .unwrap_or(false);
                if is_editing {
                    let input = text_input("", &self.editing_line.as_ref().unwrap().value)
                        .on_input(Message::LineEditChanged)
                        .on_submit(Message::LineEditSubmitted)
                        .style(edit_line_input_style)
                        .width(Length::Fill)
                        .size(11);
                    content_col = content_col.push(input);
                } else {
                    let line_btn = button(
                        container(
                            text(&cl.text).color(FG_DIM).size(11)
                                .wrapping(text::Wrapping::WordOrGlyph)
                        )
                        .width(Length::Fill)
                    )
                        .on_press(Message::LineEditStarted(cl.line_index, false, false))
                        .style(content_line_style)
                        .width(Length::Fill)
                        .padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 });
                    content_col = content_col.push(line_btn);
                }
            }
            if has_more {
                let remaining = total - 4;
                content_col = content_col.push(
                    text(format!("  ··· {} líneas más", remaining)).color(MUTED).size(11)
                );
            }

            panel_col = panel_col.push(content_col);

            if !self.expanded_tasks.is_empty() {
                panel_col = panel_col.push(
                    container(Space::new().height(Length::Fixed(1.0)))
                        .style(separator_style)
                        .width(Length::Fill)
                );
            }
        }

        // Section B — tasks
        let mut tasks_col: iced::widget::Column<'_, Message, Theme> = column![].spacing(2);

        if self.expanded_tasks.is_empty() {
            tasks_col = tasks_col.push(
                text("sin tareas — escribe algo arriba").color(MUTED).size(11)
            );
        } else {
            for task in &self.expanded_tasks {
                let (checkbox_char, checkbox_color, text_color) = if task.done {
                    ("✓", GREEN, FG_DIM)
                } else {
                    ("□", MUTED, FG)
                };

                let checkbox = button(text(checkbox_char).color(checkbox_color).size(12))
                    .on_press(Message::TaskToggled(task.line_index))
                    .style(if task.done {
                        checkbox_done_style
                    } else {
                        checkbox_undone_style
                    })
                    .padding(Padding { top: 2.0, right: 4.0, bottom: 2.0, left: 4.0 });

                let is_editing = self.editing_line.as_ref()
                    .map(|e| e.line_index == task.line_index)
                    .unwrap_or(false);

                let task_text_el: Element<'_, Message> = if is_editing {
                    text_input("", &self.editing_line.as_ref().unwrap().value)
                        .on_input(Message::LineEditChanged)
                        .on_submit(Message::LineEditSubmitted)
                        .style(edit_line_input_style)
                        .width(Length::Fill)
                        .size(11)
                        .into()
                } else {
                    button(
                        container(
                            text(&task.text).color(text_color).size(11)
                                .wrapping(text::Wrapping::WordOrGlyph)
                        )
                        .width(Length::Fill)
                    )
                        .on_press(Message::LineEditStarted(task.line_index, true, task.done))
                        .style(content_line_style)
                        .padding(Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 })
                        .width(Length::Fill)
                        .into()
                };

                let task_row = row![]
                    .push(checkbox)
                    .push(task_text_el)
                    .spacing(6)
                    .align_y(iced::Alignment::Start);

                tasks_col = tasks_col.push(task_row);
            }
        }

        let pending = self.expanded_tasks.iter().filter(|t| !t.done).count();
        let completed = self.expanded_tasks.iter().filter(|t| t.done).count();

        let footer = row![]
            .push(text(format!("{} pend · {} completadas", pending, completed)).color(MUTED).size(10))
            .push(Space::new().width(Length::Fill))
            .push(
                button(text("abrir en Canvas →").color(PURPLE).size(10))
                    .on_press(Message::TuiForNote(note_name))
                    .style(footer_button_style)
                    .padding(Padding::new(2.0))
            )
            .width(Length::Fill)
            .align_y(iced::Alignment::Center);

        let inner = panel_col
            .push(tasks_col)
            .push(Space::new().height(Length::Fixed(6.0)))
            .push(footer);

        let scroll = scrollable(inner)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(3.0)
                    .scroller_width(3.0)
            ))
            .style(scrollbar_style)
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        container(scroll)
            .style(expanded_panel_style)
            .width(Length::Fill)
            .padding(Padding::new(6.0))
            .into()
    }

    fn view_hints(&self) -> Element<'_, Message> {
        let hints = match self.mode {
            OverlayMode::Normal => {
                row![]
                    .push(hint_kbd("Enter", "enviar"))
                    .push(text(" · ").color(MUTED).size(11))
                    .push(hint_kbd("Esc", "limpiar"))
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
            }
            OverlayMode::CreatingNote => {
                row![]
                    .push(hint_kbd("Enter", "crear"))
                    .push(text(" · ").color(MUTED).size(11))
                    .push(hint_kbd("Esc", "cancelar"))
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
            }
        };

        container(hints)
            .center_x(Length::Fill)
            .padding(Padding::new(4.0))
            .height(Length::Fixed(32.0))
            .into()
    }

    fn view_ctx_menu(&self) -> Element<'_, Message> {
        let ctx = match &self.ctx_menu {
            Some(c) => c,
            None => return Space::new().into(),
        };

        let name = &ctx.note_name;
        let is_pinned = self.pinned.contains(name);
        let pin_label = if is_pinned { "📌 Desfijar" } else { "📌 Fijar arriba" };

        let mut col: iced::widget::Column<'_, Message, Theme> = column![].spacing(2);

        if ctx.confirming_delete {
            col = col
                .push(text("¿Eliminar nota?").color(RED).size(12))
                .push(
                    row![]
                        .push(button(text("Sí, eliminar").color(RED).size(11))
                            .on_press(Message::CtxConfirmDelete(name.clone()))
                            .style(ctx_button_style)
                            .padding(Padding::new(4.0)))
                        .push(button(text("No").color(FG).size(11))
                            .on_press(Message::CtxCancel)
                            .style(ctx_button_style)
                            .padding(Padding::new(4.0)))
                        .spacing(6)
                );
        } else {
            col = col
                .push(text(format!("{}:", name)).color(BLUE).size(11))
                .push(ctx_action("🗺 Ver en Canvas", Message::CtxViewTui(name.clone())))
                .push(ctx_action(pin_label, Message::CtxPin(name.clone())))
                .push(ctx_action("✕ Quitar del panel", Message::CtxHide(name.clone())))
                .push(ctx_action("🗑 Eliminar nota", Message::CtxDelete(name.clone())));
        }

        container(col)
            .padding(Padding::new(8.0))
            .style(ctx_menu_style)
            .width(Length::Fixed(200.0))
            .into()
    }
}

fn ctx_action(label: &str, msg: Message) -> Element<'_, Message> {
    button(text(label).color(FG).size(11))
        .on_press(msg)
        .style(ctx_button_style)
        .width(Length::Fill)
        .padding(Padding::new(4.0))
        .into()
}

fn app_style(_: &Overlay, _: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: FG,
    }
}

fn panel_bg_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { r: 14.0 / 255.0, g: 14.0 / 255.0, b: 14.0 / 255.0, a: 0.95 })),
        border: Border { color: BORDER_CLR, width: 1.0, radius: 12.0.into() },
        ..Default::default()
    }
}

fn compact_input_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG)),
        border: Border { color: BORDER_CLR, width: 1.0, radius: 10.0.into() },
        ..Default::default()
    }
}

fn hint_kbd<'a>(key: &'a str, action: &'a str) -> Element<'a, Message> {
    let kbd = container(text(key).color(MUTED).size(10))
        .padding(Padding::new(4.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 })),
            border: Border { color: BORDER_CLR, width: 1.0, radius: 3.0.into() },
            ..Default::default()
        });
    row![kbd, text(action).color(MUTED).size(11)]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
}

fn text_input_style(_: &Theme, _: iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        icon: Color::TRANSPARENT,
        placeholder: MUTED,
        value: FG,
        selection: Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.3 },
    }
}

fn action_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 49.0 / 255.0, g: 50.0 / 255.0, b: 68.0 / 255.0, a: 0.95 },
        _ => Color { r: 30.0 / 255.0, g: 30.0 / 255.0, b: 40.0 / 255.0, a: 0.9 },
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: BORDER_CLR, width: 1.0, radius: 8.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn note_row_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 49.0 / 255.0, g: 50.0 / 255.0, b: 68.0 / 255.0, a: 0.7 },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 6.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn selected_note_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.2 },
        _ => Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.1 },
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.5 }, width: 1.0, radius: 6.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn expanded_note_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.25 },
        _ => Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.15 },
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.6 }, width: 1.0, radius: 6.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn dots_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let color = match status {
        iced::widget::button::Status::Hovered => Color { r: 1.0, g: 1.0, b: 1.0, a: 0.6 },
        _ => Color { r: 1.0, g: 1.0, b: 1.0, a: 0.25 },
    };
    iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: color,
        ..Default::default()
    }
}

fn close_x_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let color = match status {
        iced::widget::button::Status::Hovered => Color { r: 243.0 / 255.0, g: 139.0 / 255.0, b: 168.0 / 255.0, a: 1.0 },
        _ => MUTED,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: color,
        ..Default::default()
    }
}

fn expanded_panel_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(EXPANDED_BG)),
        border: Border {
            color: TASK_BORDER,
            width: 0.5,
            radius: iced::border::Radius { top_left: 0.0, top_right: 0.0, bottom_right: 8.0, bottom_left: 8.0 },
        },
        ..Default::default()
    }
}

fn separator_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.06 })),
        ..Default::default()
    }
}

fn scrollbar_style(_: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let scroller_color = match _status {
        scrollable::Status::Hovered { .. } => Color { r: 1.0, g: 1.0, b: 1.0, a: 0.3 },
        _ => Color { r: 1.0, g: 1.0, b: 1.0, a: 0.15 },
    };
    scrollable::Style {
        container: container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        },
        vertical_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(scroller_color),
                border: Border::default(),
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(scroller_color),
                border: Border::default(),
            },
        },
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Default::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

fn checkbox_done_style(_: &Theme, _: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 0.3 })),
        border: Border { color: GREEN, width: 1.0, radius: 3.0.into() },
        text_color: GREEN,
        ..Default::default()
    }
}

fn checkbox_undone_style(_: &Theme, _: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { color: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.3 }, width: 1.0, radius: 3.0.into() },
        text_color: MUTED,
        ..Default::default()
    }
}

fn content_line_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 1.0, g: 1.0, b: 1.0, a: 0.04 },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn edit_line_input_style(_: &Theme, _: iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.06 }),
        border: Border { color: Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.4 }, width: 1.0, radius: 0.0.into() },
        icon: Color::TRANSPARENT,
        placeholder: MUTED,
        value: FG,
        selection: Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.3 },
    }
}

fn footer_button_style(_: &Theme, _: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: PURPLE,
        ..Default::default()
    }
}

fn ctx_menu_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { r: 24.0 / 255.0, g: 24.0 / 255.0, b: 32.0 / 255.0, a: 0.98 })),
        border: Border { color: BLUE, width: 1.0, radius: 10.0.into() },
        ..Default::default()
    }
}

fn ctx_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 49.0 / 255.0, g: 50.0 / 255.0, b: 68.0 / 255.0, a: 0.95 },
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn format_note_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("inbox-") {
        let rest = rest.trim_end_matches(".md");
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(rest, "%Y-%m-%d-%Hh%M") {
            let today = chrono::Local::now().date_naive();
            let date = dt.date();
            let day_label = if date == today {
                "hoy"
            } else if date == today - chrono::Duration::days(1) {
                "ayer"
            } else {
                &date.format("%d/%m").to_string()
            };
            return format!("📥 {} {}", day_label, dt.format("%Hh%M"));
        }
    }
    if name.len() > 20 {
        format!("{}…", &name[..19])
    } else {
        name.to_string()
    }
}

fn relative_time(ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(ts);
    if diff < 60 { return "ahora".to_string(); }
    if diff < 3600 { return format!("hace {}m", diff / 60); }
    if diff < 86400 { return format!("hace {}h", diff / 3600); }
    if diff < 172800 { return "ayer".to_string(); }
    format!("hace {}d", diff / 86400)
}

fn resize_zone(edge: ResizeEdge) -> Element<'static, Message> {
    let (w, h) = match edge {
        ResizeEdge::N  => (Length::Fill, Length::Fixed(RESIZE_ZONE)),
        ResizeEdge::S  => (Length::Fill, Length::Fixed(RESIZE_ZONE)),
        ResizeEdge::E  => (Length::Fixed(RESIZE_ZONE), Length::Fill),
        ResizeEdge::W  => (Length::Fixed(RESIZE_ZONE), Length::Fill),
        ResizeEdge::NE => (Length::Fixed(RESIZE_CORNER), Length::Fixed(RESIZE_CORNER)),
        ResizeEdge::NW => (Length::Fixed(RESIZE_CORNER), Length::Fixed(RESIZE_CORNER)),
        ResizeEdge::SE => (Length::Fixed(RESIZE_CORNER), Length::Fixed(RESIZE_CORNER)),
        ResizeEdge::SW => (Length::Fixed(RESIZE_CORNER), Length::Fixed(RESIZE_CORNER)),
    };

    mouse_area(
        container(Space::new().width(w).height(h))
            .style(resize_zone_style)
    )
    .on_press(Message::ResizeStarted(edge))
    .on_release(Message::ResizeEnded)
    .into()
}

fn resize_zone_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        ..Default::default()
    }
}

fn main() -> iced_layershell::Result {
    std::env::set_var("WAYLAND_DEBUG", "0");

    let panel = load_panel();
    let init_w = if panel.window_w > 0 { panel.window_w.clamp(MIN_W, MAX_W) } else { 428 };
    let init_h = if panel.window_h > 0 { panel.window_h.clamp(MIN_H, MAX_H) } else { 280 };

    application(
        Overlay::new,
        "overlay-iced",
        Overlay::update,
        Overlay::view,
    )
    .subscription(Overlay::subscription)
    .layer_settings(LayerShellSettings {
        size: Some((init_w, init_h)),
        anchor: Anchor::Top | Anchor::Right,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        exclusive_zone: 0,
        ..Default::default()
    })
    .style(app_style)
    .run()
}
