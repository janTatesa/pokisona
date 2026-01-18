#![deny(clippy::disallowed_types)]
mod cli;
mod command;
mod command_history;
mod config;
mod file_store;
mod markdown;
mod pane_state;
mod view;

use std::rc::Rc;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use iced::{
    Event, Point, Task, event, exit, mouse,
    widget::{
        Id,
        operation::focus,
        pane_grid::{self, Axis, Pane}
    },
    window
};
use lucide_icons::LUCIDE_FONT_BYTES;
use url::Url;

use crate::{
    cli::{InitialFile, VaultName},
    command::Command,
    command_history::CommandHistory,
    config::{Config, Keybinding},
    file_store::{FileData, FileStore},
    markdown::store::MarkdownStore,
    pane_state::PaneState,
    view::Theme
};

type Element<'a> = iced::Element<'a, Message, Theme>;
struct Pokisona {
    config: Config,

    vault_name: String,

    scale: f32,

    file_store: &'static FileStore,

    command_history: CommandHistory,
    typed_command: Option<String>,

    error: Option<String>,

    panes: pane_grid::State<PaneState>,
    focus: pane_grid::Pane,

    hovered_link: Option<HoveredLink>,
    mouse_pos: Point
}

enum HoveredLink {
    Internal(Rc<FileData>),
    Error(String),
    External(String)
}

/// Most [`Message`]s should be put in [`Command`], see it's documentation
#[derive(Debug, Clone)]
enum Message {
    Focus(ElementId),

    Hover(Link),
    HoverEnd,
    MouseMoved(Point),

    KeyEvent(Keybinding),
    FileOpened { path: PathBuf, content: String },

    Command(Command)
}

#[derive(Debug, Clone)]
enum Link {
    InvalidUrlExternal(String),
    External(Url),

    Internal(PathBuf),
    /// Currently handled the same as [`Link::InvalidUrlExternal`], in future will create a new file on click
    NonExistentInternal(PathBuf)
}

impl Message {
    fn from_iced_event(event: Event, _: event::Status, _: window::Id) -> Option<Self> {
        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Self::MouseMoved(position))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Back)) => {
                Some(Command::FileHistoryBackward.into())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Forward)) => {
                Some(Command::FileHistoryForward.into())
            }
            Event::Keyboard(event) => Keybinding::from_iced_key_event(event).map(Self::KeyEvent),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ElementId {
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
    fn theme(&self) -> Theme {
        self.config.theme
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::FileOpened { path, content } => {
                self.file_store.insert(&path, MarkdownStore::new(content))
            }
            Message::Focus(id) => return focus(id),
            Message::KeyEvent(event) => {
                if let Some(command) = self.config.keybindings.get(&event) {
                    return self.handle_command(command.clone());
                }
            }
            Message::Hover(link) => {
                self.hovered_link = Some(match link {
                    Link::InvalidUrlExternal(raw) => HoveredLink::Error(raw),
                    Link::NonExistentInternal(path) => HoveredLink::Error(path.into_string()),
                    Link::Internal(path) => {
                        let (data, task) = self.file_store.open_file(path);
                        self.hovered_link = Some(HoveredLink::Internal(data));
                        return task;
                    }
                    Link::External(url) => HoveredLink::External(url.to_string())
                });
            }
            Message::HoverEnd => self.hovered_link = None,
            Message::MouseMoved(point) => self.mouse_pos = point,
            Message::Command(command) => {
                self.error = None;
                return self.handle_command(command);
            }
        }

        Task::none()
    }

    fn handle_command(&mut self, command: Command) -> Task<Message> {
        let scale = self.config.scale;
        match command {
            Command::CommandLineSet(command) => {
                self.command_history.deselect();
                self.typed_command = Some(command)
            }
            Command::Follow(link) => {
                return match link {
                    Link::Internal(path) => self.handle_command(Command::Open { path }),
                    Link::External(url) => open::that(url.as_str())
                        .map(|_| Task::none())
                        .unwrap_or_else(|error| {
                            Task::done(Command::Error(error.to_string()).into())
                        }),
                    _ => Task::none()
                };
            }
            Command::Error(error) => self.error = Some(error),
            Command::CommandLineSubmit => {
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
                        self.error = Some(format!(
                            "Error parsing command \"{typed_command}\": {error}"
                        ));

                        return Task::none();
                    }
                };

                self.command_history.push(typed_command.to_owned());
                return self.handle_command(command);
            }
            Command::Quit(pane) => {
                let pane = pane.unwrap_or(self.focus);

                let Some((_state, pane)) = self.panes.close(pane) else {
                    return exit();
                };

                self.focus = pane;
            }
            Command::QuitAll => return exit(),
            Command::Open { mut path } => {
                if path.extension().is_none() {
                    path.set_extension("md");
                }

                let (file, task) = self.file_store.open_file(path);
                self.panes.get_mut(self.focus).unwrap().open(file);
                return task;
            }
            Command::VSplit { path, pane } => return self.split(path, pane, Axis::Vertical),
            Command::HSplit { path, pane } => return self.split(path, pane, Axis::Horizontal),
            Command::ScaleUp => self.scale += scale.default_step,
            Command::ScaleDown => {
                let scale = self.scale - scale.default_step;
                if scale > 0. {
                    self.scale = scale;
                }
            }
            Command::ScaleReset => self.scale = scale.default,
            Command::HistoryUp => self.command_history.select_up(),
            Command::HistoryDown => self.command_history.select_down(),
            Command::CommandModeOpen => self.typed_command = Some(String::new()),
            Command::Noop => {}
            Command::CommandModeExit => self.typed_command = None,
            Command::FocusPane(pane) => self.focus = pane,
            Command::DropPane { pane, target } => self.panes.drop(pane, target),
            Command::ResizePane(resize_event) => {
                self.panes.resize(resize_event.split, resize_event.ratio)
            }
            Command::FocusAdjacent(direction) => {
                if let Some(pane) = self.panes.adjacent(self.focus, direction) {
                    self.focus = pane
                }
            }
            Command::FileHistoryForward => self.panes.get_mut(self.focus).unwrap().forward(),
            Command::FileHistoryBackward => self.panes.get_mut(self.focus).unwrap().backward()
        }

        Task::none()
    }

    fn split(
        &mut self,
        path: Option<Utf8PathBuf>,
        pane: Option<Pane>,
        axis: Axis
    ) -> Task<Message> {
        let pane = pane.unwrap_or(self.focus);

        let (state, task) = match path {
            Some(path) => {
                let (file, task) = self.file_store.open_file(path);
                (PaneState::new(file), task)
            }
            None => (
                self.panes
                    .get(pane)
                    .map(PaneState::split)
                    .unwrap_or_default(),
                Task::none()
            )
        };

        self.focus = self.panes.split(axis, pane, state).unwrap().0;
        task
    }
}

type PathBuf = Utf8PathBuf;
type Path = Utf8Path;
fn main() -> Result<()> {
    color_eyre::install()?;
    let (VaultName(vault_name), InitialFile(initial_file), config) = cli::handle_args()?;
    iced::application(
        move || {
            let file_store = Box::leak(Box::new(FileStore::default()));
            let (first_pane_state, task) = match initial_file.clone() {
                Some(initial_file) => {
                    let (data, task) = file_store.open_file(initial_file);
                    (PaneState::new(data), task)
                }
                None => (PaneState::default(), Task::none())
            };

            let (panes, focus) = pane_grid::State::new(first_pane_state);

            let app = Pokisona {
                config: config.clone(),
                vault_name: vault_name.clone(),
                typed_command: None,
                error: None,
                command_history: CommandHistory::default(),
                scale: config.scale.default,
                hovered_link: None,
                file_store,
                mouse_pos: Point::ORIGIN,
                panes,
                focus
            };

            (app, task)
        },
        Pokisona::update,
        Pokisona::view
    )
    .font(LUCIDE_FONT_BYTES)
    .theme(Pokisona::theme)
    .subscription(|_| event::listen_with(Message::from_iced_event))
    .scale_factor(|app| app.scale)
    .run()?;
    Ok(())
}
