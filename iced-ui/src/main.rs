use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Background, Border, Color, Element, Event, Length, Padding, Point, Task, Theme};
use iced_layershell::build_pattern::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;

use overlay_core::{append_to_note, get_recent_notes, load_config_embedded, open_tui, read_note};

const CONFIG_TOML: &str = include_str!("../../src-tauri/config.toml");

const BG: Color = Color { r: 18.0 / 255.0, g: 18.0 / 255.0, b: 18.0 / 255.0, a: 0.92 };
const FG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.85 };
const MUTED: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.35 };
const GREEN: Color = Color { r: 29.0 / 255.0, g: 158.0 / 255.0, b: 117.0 / 255.0, a: 1.0 };
const PURPLE_BG: Color = Color { r: 83.0 / 255.0, g: 74.0 / 255.0, b: 183.0 / 255.0, a: 0.15 };
const BORDER_CLR: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 };
const BLUE: Color = Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.8 };
const DRAG_HINT: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.15 };

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    InputSubmitted,
    NoteClicked(usize),
    TuiClicked,
    EscapePressed,
    DragStarted,
    DragMoved(f32, f32),
    DragEnded,
}

struct Overlay {
    input_value: String,
    notes: Vec<overlay_core::NoteInfo>,
    notes_dir: std::path::PathBuf,
    status_msg: String,
    selected_file: String,
    expanded: bool,
    expanded_content: String,
    expanded_name: String,
    dragging: bool,
    drag_origin: Point,
    drag_margin_start: (i32, i32, i32, i32),
    margin: (i32, i32, i32, i32),
    last_cursor: Point,
}

impl Overlay {
    fn new() -> Self {
        let config = load_config_embedded(CONFIG_TOML);
        let notes_dir = overlay_core::resolve_notes_path(&config);
        let notes = get_recent_notes(&notes_dir);
        Overlay {
            input_value: String::new(),
            notes,
            notes_dir,
            status_msg: String::new(),
            selected_file: "inbox.md".to_string(),
            expanded: false,
            expanded_content: String::new(),
            expanded_name: String::new(),
            dragging: false,
            drag_origin: Point::default(),
            drag_margin_start: (0, 0, 0, 0),
            margin: (0, 0, 0, 0),
            last_cursor: Point::default(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
                Task::none()
            }
            Message::InputSubmitted => {
                let trimmed = self.input_value.trim().to_string();
                if !trimmed.is_empty() {
                    match append_to_note(&self.notes_dir, &self.selected_file, &trimmed) {
                        Ok(_) => {
                            self.status_msg = "✓".to_string();
                            self.input_value.clear();
                            self.notes = get_recent_notes(&self.notes_dir);
                        }
                        Err(e) => {
                            self.status_msg = format!("✗ {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::NoteClicked(idx) => {
                if idx < self.notes.len() {
                    let note = &self.notes[idx];
                    let filename = format!("{}.md", note.name);
                    self.selected_file = filename.clone();
                    match read_note(&self.notes_dir, &filename) {
                        Ok(content) => {
                            self.expanded_content = content;
                            self.expanded_name = note.name.clone();
                            self.expanded = true;
                        }
                        Err(e) => {
                            self.expanded_content = format!("Error: {}", e);
                            self.expanded_name = note.name.clone();
                            self.expanded = true;
                        }
                    }
                }
                Task::none()
            }
            Message::TuiClicked => {
                let _ = open_tui();
                Task::none()
            }
            Message::EscapePressed => {
                if self.expanded {
                    self.expanded = false;
                    self.expanded_content.clear();
                    self.expanded_name.clear();
                } else {
                    self.input_value.clear();
                    self.status_msg.clear();
                    self.selected_file = "inbox.md".to_string();
                }
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
        if self.expanded {
            self.view_expanded()
        } else {
            self.view_compact()
        }
    }

    fn view_compact(&self) -> Element<'_, Message> {
        let drag_bar = container(
            text("···").color(DRAG_HINT).size(10)
        )
        .center_x(Length::Fill)
        .height(Length::Fixed(12.0))
        .style(transparent_style);

        let prompt = text("❯").color(GREEN).size(16);
        let input = text_input("Escribe y presiona Enter...", &self.input_value)
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
        if self.selected_file != "inbox.md" {
            let label = self.selected_file.replace(".md", "");
            input_row = input_row.push(text(format!("→ {}", label)).color(BLUE).size(11));
        }

        let input_container = container(input_row)
            .padding(Padding::new(8.0))
            .style(compact_input_style)
            .width(Length::Fill);

        let mut grid_col: iced::widget::Column<'_, Message, Theme> = column![].spacing(6);

        for row_chunk in self.notes.chunks(3) {
            let mut grid_row: iced::widget::Row<'_, Message, Theme> = row![].spacing(6);
            for (i, note) in row_chunk.iter().enumerate() {
                let name = if note.name.len() > 12 {
                    format!("{}…", &note.name[..11])
                } else {
                    note.name.clone()
                };
                let sub = if note.task_count > 0 {
                    text(format!("✓ {}/{}", note.tasks_done, note.task_count)).color(GREEN).size(10)
                } else {
                    text(relative_time(note.modified)).color(MUTED).size(10)
                };
                let cell = button(
                    column![].push(text(name).color(FG).size(12)).push(sub)
                        .spacing(4).align_x(iced::Alignment::Center)
                )
                .on_press(Message::NoteClicked(i))
                .style(note_button_style)
                .width(Length::Fill);
                grid_row = grid_row.push(cell);
            }
            grid_col = grid_col.push(grid_row);
        }

        let tui_cell = button(
            column![].push(text("⌨ TUI").color(GREEN).size(12)).push(text("terminal").color(MUTED).size(10))
                .spacing(4).align_x(iced::Alignment::Center)
        )
        .on_press(Message::TuiClicked)
        .style(tui_button_style)
        .width(Length::Fill);

        let mut last_row: iced::widget::Row<'_, Message, Theme> = row![].spacing(6);
        last_row = last_row.push(tui_cell);
        for _ in 0..2 {
            last_row = last_row.push(container(text("").size(12)).width(Length::Fill));
        }
        grid_col = grid_col.push(last_row);

        let grid_container = container(grid_col)
            .height(Length::Fixed(156.0))
            .width(Length::Fill);

        let hints = row![]
            .push(hint_kbd("Enter", "enviar"))
            .push(text(" · ").color(MUTED).size(11))
            .push(hint_kbd("Esc", "limpiar"))
            .spacing(4)
            .align_y(iced::Alignment::Center);

        let hints_container = container(hints)
            .center_x(Length::Fill)
            .padding(Padding::new(4.0))
            .height(Length::Fixed(32.0));

        let content = column![]
            .push(drag_bar)
            .push(input_container)
            .push(grid_container)
            .push(hints_container)
            .spacing(4)
            .padding(Padding::new(4.0))
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .style(transparent_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_expanded(&self) -> Element<'_, Message> {
        let header = row![]
            .push(text(&self.expanded_name).color(BLUE).size(14))
            .push(Space::new().width(Length::Fill))
            .push(button(text("✕").color(MUTED).size(14))
                .on_press(Message::EscapePressed)
                .style(close_button_style))
            .align_y(iced::Alignment::Center)
            .padding(Padding::new(4.0))
            .width(Length::Fill);

        let body = container(text(&self.expanded_content).color(FG).size(12))
            .padding(Padding::new(12.0))
            .style(expanded_body_style)
            .width(Length::Fill)
            .height(Length::Fill);

        let hints = row![].push(hint_kbd("Esc", "cerrar"))
            .spacing(4).align_y(iced::Alignment::Center);
        let hints_bar = container(hints)
            .center_x(Length::Fill)
            .padding(Padding::new(4.0))
            .height(Length::Fixed(40.0));

        let layout = column![]
            .push(header).push(body).push(hints_bar)
            .spacing(6).padding(Padding::new(8.0))
            .width(Length::Fill).height(Length::Fill);

        container(layout)
            .style(transparent_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn app_style(_: &Overlay, _: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: FG,
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

fn expanded_body_style(_: &Theme) -> container::Style {
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

fn note_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 49.0 / 255.0, g: 50.0 / 255.0, b: 68.0 / 255.0, a: 0.95 },
        _ => BG,
    };
    let border_clr = match status {
        iced::widget::button::Status::Hovered => Color { r: 137.0 / 255.0, g: 180.0 / 255.0, b: 250.0 / 255.0, a: 0.4 },
        _ => BORDER_CLR,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: border_clr, width: 1.0, radius: 10.0.into() },
        text_color: FG,
        ..Default::default()
    }
}

fn tui_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let a = match status { iced::widget::button::Status::Hovered => 0.5, _ => 0.25 };
    iced::widget::button::Style {
        background: Some(Background::Color(PURPLE_BG)),
        border: Border { color: Color { r: 166.0 / 255.0, g: 227.0 / 255.0, b: 161.0 / 255.0, a }, width: 1.0, radius: 10.0.into() },
        text_color: GREEN,
        ..Default::default()
    }
}

fn close_button_style(_: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 49.0 / 255.0, g: 50.0 / 255.0, b: 68.0 / 255.0, a: 0.8 },
        _ => Color::TRANSPARENT,
    };
    let tc = match status {
        iced::widget::button::Status::Hovered => Color { r: 243.0 / 255.0, g: 139.0 / 255.0, b: 168.0 / 255.0, a: 1.0 },
        _ => MUTED,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 4.0.into() },
        text_color: tc,
        ..Default::default()
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
