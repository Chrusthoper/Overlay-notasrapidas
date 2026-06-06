use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Background, Border, Color, Element, Event, Length, Padding, Point, Task, Theme};
use iced_layershell::build_pattern::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;
use serde::{Deserialize, Serialize};

use overlay_core::{append_to_note, get_recent_notes, load_config_embedded, open_tui, open_tui_with_file};

const CONFIG_TOML: &str = include_str!("../../src-tauri/config.toml");

const BG: Color = Color { r: 18.0 / 255.0, g: 18.0 / 255.0, b: 18.0 / 255.0, a: 0.92 };
const FG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.85 };
const MUTED: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const GREEN: Color = Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 1.0 };
const PURPLE_BG: Color = Color { r: 83.0 / 255.0, g: 74.0 / 255.0, b: 183.0 / 255.0, a: 0.15 };
const BORDER_CLR: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 };
const BLUE: Color = Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.8 };
const DRAG_HINT: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.15 };
const RED: Color = Color { r: 243.0 / 255.0, g: 139.0 / 255.0, b: 168.0 / 255.0, a: 1.0 };
const ORANGE: Color = Color { r: 250.0 / 255.0, g: 179.0 / 255.0, b: 135.0 / 255.0, a: 1.0 };

#[derive(Debug, Clone, Copy, PartialEq)]
enum OverlayMode {
    Normal,
    CreatingNote,
}

#[derive(Debug, Clone)]
struct CtxMenu {
    note_name: String,
    confirming_delete: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct PanelState {
    #[serde(default)]
    pinned: Vec<String>,
    #[serde(default)]
    hidden: Vec<String>,
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

fn save_panel(pinned: &[String], hidden: &[String]) {
    let path = panel_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let state = PanelState {
        pinned: pinned.to_vec(),
        hidden: hidden.to_vec(),
    };
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&state).unwrap_or_default());
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    InputSubmitted,
    NoteClicked(usize),
    NoteCtxClicked(usize),
    TuiClicked,
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
    dragging: bool,
    drag_origin: Point,
    drag_margin_start: (i32, i32, i32, i32),
    margin: (i32, i32, i32, i32),
    last_cursor: Point,
    window_height: u32,
}

impl Overlay {
    fn new() -> Self {
        let config = load_config_embedded(CONFIG_TOML);
        let notes_dir = overlay_core::resolve_notes_path(&config);
        let notes = get_recent_notes(&notes_dir);
        let now = chrono::Local::now();
        let session_file = format!("inbox-{}.md", now.format("%Y-%m-%d-%Hh%M"));
        let panel = load_panel();
        let window_height = 248;
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
            dragging: false,
            drag_origin: Point::default(),
            drag_margin_start: (0, 0, 0, 0),
            margin: (0, 0, 0, 0),
            last_cursor: Point::default(),
            window_height,
        }
    }

    fn visible_notes(&self) -> Vec<&overlay_core::NoteInfo> {
        self.notes
            .iter()
            .filter(|n| !self.hidden.contains(&n.name))
            .collect()
    }

    fn sorted_visible_notes(&self) -> Vec<&overlay_core::NoteInfo> {
        let mut vis: Vec<&overlay_core::NoteInfo> = self.visible_notes();
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
        let list_height = note_count * 36 + 8;
        let total = 40 + 40 + list_height + 32;
        total.clamp(248, 480)
    }

    fn refresh_notes(&mut self) {
        self.notes = get_recent_notes(&self.notes_dir);
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
                    self.selected_file = format!("{}.md", notes[idx].name);
                }
                Task::none()
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
            Message::TuiClicked => {
                let _ = open_tui();
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
                if self.ctx_menu.is_some() {
                    self.ctx_menu = None;
                } else if self.mode == OverlayMode::CreatingNote {
                    self.mode = OverlayMode::Normal;
                    self.input_value.clear();
                } else {
                    self.input_value.clear();
                    self.status_msg.clear();
                    self.selected_file = self.session_file.clone();
                }
                Task::none()
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
                save_panel(&self.pinned, &self.hidden);
                self.ctx_menu = None;
                Task::none()
            }
            Message::CtxHide(name) => {
                self.hidden.push(name);
                save_panel(&self.pinned, &self.hidden);
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
                self.pinned.retain(|n| n != &name);
                self.hidden.retain(|n| n != &name);
                save_panel(&self.pinned, &self.hidden);
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
                self.drag_origin = self.last_cursor;
                self.drag_margin_start = self.margin;
                Task::none()
            }
            Message::DragMoved(x, y) => {
                self.last_cursor = Point { x, y };
                if self.dragging {
                    let dx = (x - self.drag_origin.x) as i32;
                    let dy = (y - self.drag_origin.y) as i32;
                    self.margin = (
                        self.drag_margin_start.0 + dy,
                        self.drag_margin_start.1 - dx,
                        0,
                        0,
                    );
                    Task::done(Message::MarginChange(self.margin))
                } else {
                    Task::none()
                }
            }
            Message::DragEnded => {
                self.dragging = false;
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

            Event::Mouse(MouseEvent::ButtonPressed(mouse::Button::Left)) => {
                Some(Message::DragStarted)
            }

            Event::Mouse(MouseEvent::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::DragEnded)
            }

            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let mut content = column![]
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

        container(content)
            .style(panel_bg_style)
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
                        button(text("⌨ TUI").color(GREEN).size(11))
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
        let mut col: iced::widget::Column<'_, Message, Theme> = column![].spacing(2);

        for (idx, note) in notes.iter().enumerate() {
            let filename = format!("{}.md", note.name);
            let is_selected = filename == self.selected_file;
            let is_session = filename == self.session_file;
            let is_pinned = self.pinned.contains(&note.name);

            let dot_color = if is_selected {
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
            .style(if is_selected {
                selected_note_style
            } else {
                note_row_style
            })
            .width(Length::Fill)
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 });

            let ctx_btn = button(text("⋮").size(10))
                .on_press(Message::NoteCtxClicked(idx))
                .style(ctx_inline_style)
                .padding(Padding::new(2.0));

            let row_el = row![]
                .push(text("●").color(dot_color).size(8))
                .push(cell)
                .push(ctx_btn)
                .spacing(4)
                .align_y(iced::Alignment::Center);

            col = col.push(row_el);
        }

        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_hints(&self) -> Element<'_, Message> {
        let hints = match self.mode {
            OverlayMode::Normal => {
                row![]
                    .push(hint_kbd("Enter", "enviar"))
                    .push(text(" · ").color(MUTED).size(11))
                    .push(hint_kbd("Esc", "limpiar"))
                    .push(text(" · ").color(MUTED).size(11))
                    .push(hint_kbd("clic der", "opciones"))
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
                .push(ctx_action("👁 Ver en TUI", Message::CtxViewTui(name.clone())))
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

fn transparent_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
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

fn ctx_inline_style(_: &Theme, _: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: MUTED,
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

fn main() -> iced_layershell::Result {
    std::env::set_var("WAYLAND_DEBUG", "0");

    application(
        Overlay::new,
        "overlay-iced",
        Overlay::update,
        Overlay::view,
    )
    .subscription(Overlay::subscription)
    .layer_settings(LayerShellSettings {
        size: Some((428, 248)),
        anchor: Anchor::Top | Anchor::Right,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        exclusive_zone: 0,
        ..Default::default()
    })
    .style(app_style)
    .run()
}
