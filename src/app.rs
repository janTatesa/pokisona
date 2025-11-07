use std::{rc::Rc, time::Duration};

use color_eyre::Result;
use iced::{
    Alignment::{self},
    Event, Length, Padding, Point, Task, Vector, event, exit, mouse,
    widget::{self, Id, column, operation::focus, row, stack},
    window
};
use smol::{Timer, fs};

use crate::{
    PathBuf,
    command::{Command, CommandKind},
    command_history::CommandHistory,
    config::{Config, Keybinding},
    file_store::{FileData, FileStore},
    iced_helpers::{BORDER_WIDTH, BorderType, Element, Link, SPACING, container, text},
    markdown::Markdown,
    theme::Theme,
    window::{Direction, Window, WindowManager}
};

pub struct Pokisona {
    config: Config,

    vault_name: String,
    window_manager: WindowManager,

    scale: f32,

    file_store: &'static FileStore,

    command_history: CommandHistory,
    typed_command: Option<String>,

    error_id: u64,
    error: Option<String>,

    hovered_link: Option<HoveredLink>,
    mouse_pos: Point
}

enum HoveredLink {
    Internal(Rc<FileData>),
    Error(String),
    External(String)
}

#[derive(Debug, Clone)]
pub enum Message {
    InitialFileOpen(PathBuf),

    EditCommand(String),
    SubmitCommand,
    Focus(ElementId),

    LinkClick(Link),
    Hover(Link),
    HoverEnd,
    MouseMoved(Point),

    KeyEvent(Keybinding),
    ClearError(u64),
    FileOpened { path: PathBuf, content: String },
    Error(String)
}

impl Message {
    fn from_iced_event(event: Event, _: event::Status, _: window::Id) -> Option<Self> {
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            return Some(Self::MouseMoved(position));
        };

        if let Event::Keyboard(event) = event
            && let Some(keybinding) = Keybinding::from_iced_key_event(event)
        {
            return Some(Self::KeyEvent(keybinding));
        }

        None
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ElementId {
    CommandInput
}

impl From<ElementId> for Id {
    fn from(val: ElementId) -> Self {
        Self::new(match val {
            ElementId::CommandInput => "command-input"
        })
    }
}

impl Pokisona {
    pub fn run(
        vault_name: String,
        initial_file: Option<PathBuf>,
        config: Config
    ) -> Result<(), iced::Error> {
        iced::application(
            move || {
                let task = match initial_file.clone() {
                    Some(initial_file) => Task::done(Message::InitialFileOpen(initial_file)),
                    None => Task::none()
                };

                let app = Self {
                    config: config.clone(),
                    vault_name: vault_name.clone(),
                    window_manager: WindowManager::default(),
                    typed_command: None,
                    error: None,
                    error_id: 0,
                    command_history: CommandHistory::default(),
                    scale: config.scale.default,
                    hovered_link: None,
                    file_store: Box::leak(Box::new(FileStore::default())),
                    mouse_pos: Point::ORIGIN
                };

                (app, task)
            },
            Self::update,
            Self::view
        )
        .theme(Self::theme)
        .subscription(|_| event::listen_with(Message::from_iced_event))
        .scale_factor(|app| app.scale)
        .run()
    }

    fn theme(&self) -> Theme {
        self.config.theme
    }

    fn open_file(&mut self, path: PathBuf) -> (Rc<FileData>, Task<Message>) {
        let (data, newly_created) = self.file_store.get_ref(path.clone());

        let task = if newly_created {
            Task::future(async {
                let content = fs::read_to_string(&path)
                    .await
                    .map_err(|error| error.to_string());
                content
                    .map(|content| Message::FileOpened { path, content })
                    .unwrap_or_else(Message::Error)
            })
        } else {
            Task::none()
        };

        (data, task)
    }

    const ERROR_DISPLAY_DURATION: Duration = Duration::from_secs(1);
    fn display_error(&mut self, error: String) -> Task<Message> {
        self.error_id += 1;
        self.error = Some(error);
        let id = self.error_id;
        Task::future(async move {
            Timer::after(Self::ERROR_DISPLAY_DURATION).await;
            Message::ClearError(id)
        })
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::EditCommand(command) => {
                self.command_history.deselect();
                self.typed_command = Some(command)
            }
            Message::FileOpened { path, content } => {
                self.file_store.insert(&path, Markdown::new(content))
            }
            Message::Error(error) => {
                return self.display_error(error);
            }
            Message::SubmitCommand => {
                let typed_command = self.typed_command.take();
                let typed_command = self
                    .command_history
                    .currently_selected()
                    .or(typed_command.as_deref())
                    .unwrap_or_default();
                if typed_command.is_empty() {
                    return Task::none();
                }

                let command: Command = match typed_command.parse() {
                    Ok(command) => command,
                    Err(error) => {
                        let error = format!("Error parsing command \"{typed_command}\": {error}");
                        return self.display_error(error);
                    }
                };

                self.command_history.push(typed_command.to_owned());
                return self.handle_command(command);
            }
            Message::Focus(id) => return focus(id),
            Message::ClearError(id) if self.error_id == id => self.error = None,
            Message::ClearError(_) => {}
            Message::InitialFileOpen(path) => {
                return self.handle_command(Command {
                    _force: false,
                    kind: CommandKind::Open { path }
                });
            }
            Message::KeyEvent(event) => {
                if let Some(command) = self.config.keybindings.get(&event) {
                    return self.handle_command(command.clone());
                }
            }
            Message::LinkClick(link) => {
                return match link {
                    Link::Internal(path) => self.handle_command(Command {
                        _force: false,
                        kind: CommandKind::Open { path }
                    }),
                    Link::External(url) => open::that(url.as_str())
                        .map(|_| Task::none())
                        .unwrap_or_else(|error| Task::done(Message::Error(error.to_string()))),
                    _ => Task::none()
                };
            }
            Message::Hover(link) => {
                self.hovered_link = Some(match link {
                    Link::InvalidUrlExternal(raw) => HoveredLink::Error(raw),
                    Link::NonExistentInternal(path) => HoveredLink::Error(path.into_string()),
                    Link::Internal(path) => {
                        let (data, task) = self.open_file(path);
                        self.hovered_link = Some(HoveredLink::Internal(data));
                        return task;
                    }
                    Link::External(url) => HoveredLink::External(url.to_string())
                });
            }
            Message::HoverEnd => self.hovered_link = None,
            Message::MouseMoved(point) => self.mouse_pos = point
        }

        Task::none()
    }

    fn handle_command(&mut self, command: Command) -> Task<Message> {
        let scale = self.config.scale;
        match command.kind {
            CommandKind::Quit => {
                if self.window_manager.remove_window().is_none() {
                    return exit();
                }
            }
            CommandKind::QuitAll => return exit(),
            CommandKind::Open { path } => {
                let (data, task) = self.open_file(path);
                *self.window_manager.current_window_mut() = Window::Markdown(data);
                return task;
            }

            CommandKind::Split { path } => {
                let (window, task) = match path {
                    Some(path) => {
                        let (data, task) = self.open_file(path);
                        (Window::Markdown(data), task)
                    }
                    None => (Window::Empty, Task::none())
                };

                self.window_manager.split(window);
                return task;
            }
            CommandKind::VSplit { path } => {
                let (window, task) = match path {
                    Some(path) => {
                        let (data, task) = self.open_file(path);
                        (Window::Markdown(data), task)
                    }
                    None => (Window::Empty, Task::none())
                };

                self.window_manager
                    .split_at_direction(window, Direction::Vertical);
                return task;
            }
            CommandKind::HSplit { path } => {
                let (window, task) = match path {
                    Some(path) => {
                        let (data, task) = self.open_file(path);
                        (Window::Markdown(data), task)
                    }
                    None => (Window::Empty, Task::none())
                };

                self.window_manager
                    .split_at_direction(window, Direction::Horizontal);
                return task;
            }

            CommandKind::TransposeWindows => self.window_manager.transpose_windows(),
            CommandKind::NextWindow => self.window_manager.next_window(),
            CommandKind::PreviousWindow => self.window_manager.previous_window(),

            CommandKind::ScaleUp => self.scale += scale.default_step,
            CommandKind::ScaleDown => {
                let scale = self.scale - scale.default_step;
                if scale > 0. {
                    self.scale = scale;
                }
            }
            CommandKind::ScaleReset => self.scale = scale.default,

            CommandKind::HistoryUp => self.command_history.select_up(),
            CommandKind::HistoryDown => self.command_history.select_down(),
            CommandKind::CommandModeOpen => self.typed_command = Some(String::new()),
            CommandKind::Noop => {}
            CommandKind::CommandModeExit => self.typed_command = None
        }

        Task::none()
    }

    fn view(&self) -> Element<'_> {
        let theme = self.config.theme;
        let vault_name = container(self.vault_name.as_str())
            .align_x(Alignment::End)
            .width(Length::Fill);

        let bar_content: Element<'_> = match (
            self.command_history.currently_selected(),
            self.typed_command.as_deref(),
            &self.error
        ) {
            (Some(command), ..) | (_, Some(command), _) => row![
                ":",
                widget::sensor(
                    widget::text_input("Enter command", command)
                        .on_input(Message::EditCommand)
                        .on_submit(Message::SubmitCommand)
                        .id(ElementId::CommandInput)
                        .padding(0)
                )
                .on_show(|_| Message::Focus(ElementId::CommandInput)),
                vault_name
            ]
            .spacing(0)
            .into(),
            (_, _, Some(error)) => row![text(error, theme.danger), vault_name].into(),
            _ => vault_name.into()
        };

        let bar = container(bar_content).width(Length::Fill);
        let hovered_link = self.hovered_link.as_ref().and_then(|link| {
            let hovered_link = match link {
                HoveredLink::Internal(file_data) => {
                    let bar = container(file_data.path().as_str())
                        .align_x(Alignment::Center)
                        .border(BorderType::TitleBarBottom)
                        .custom_padding(Padding::default().left(SPACING).right(SPACING))
                        .color(theme.crust);
                    let content = container(file_data.content()?.inner().render(theme))
                        .custom_padding(SPACING - BORDER_WIDTH);
                    container(column![content, bar].padding(BORDER_WIDTH))
                }
                HoveredLink::Error(url) => container(text(url, theme.danger)).padded(),
                HoveredLink::External(url) => container(text(url, theme.link_external)).padded()
            }
            .color(theme.base)
            .border(BorderType::Normal);
            const CURSOR_HOVER_OFFSET: f32 = 8.;
            let hovered_link_pos =
                self.mouse_pos + Vector::new(CURSOR_HOVER_OFFSET, CURSOR_HOVER_OFFSET);
            let padding = Padding::default()
                .top(hovered_link_pos.y)
                .left(hovered_link_pos.x);
            let hovered_link = container(hovered_link).custom_padding(padding);
            Some(hovered_link)
        });

        let windows = container(self.window_manager.render(self.theme()))
            .padded()
            .stretched();
        let ui = column![windows, bar];
        stack![ui, hovered_link].into()
    }
}
