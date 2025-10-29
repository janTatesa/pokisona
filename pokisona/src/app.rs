use std::{path::PathBuf, time::Duration};

use color_eyre::Result;
use iced::{
    Alignment::{self},
    Element, Event, Task, event, exit,
    widget::{Id, operation::focus},
    window
};
use smol::{Timer, fs};

use crate::{
    column,
    command::{Command, CommandKind},
    command_history::CommandHistory,
    config::{Config, Keybinding},
    file_store::FILE_STORE,
    markdown_store::MarkdownStore,
    row,
    widget::{ContainerKind, Spacing, Theme, Widget},
    window::{Window, WindowManager}
};

pub struct Pokisona {
    config: Config,

    vault_name: String,
    vault_path: PathBuf,
    window_manager: WindowManager,

    scale: f32,

    command_history: CommandHistory,
    typed_command: Option<String>,

    error_id: u64,
    error: Option<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    InitialFileOpen(PathBuf),
    Type(TextInputId, String),
    Submit(TextInputId),
    Focus(Id),
    KeyEvent(Keybinding),
    ClearError(u64),
    FileOpened {
        path: PathBuf,
        // HACK: We have to use string because message has to be clone
        content: Result<String, String>
    }
}

impl Message {
    fn from_iced_event(event: Event, _: event::Status, _: window::Id) -> Option<Self> {
        if let Event::Keyboard(event) = event
            && let Some(keybinding) = Keybinding::from_iced_key_event(event)
        {
            return Some(Self::KeyEvent(keybinding));
        }

        None
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextInputId {
    CommandInput
}

impl From<TextInputId> for Id {
    fn from(val: TextInputId) -> Self {
        Self::new(match val {
            TextInputId::CommandInput => "command-input"
        })
    }
}

impl Pokisona {
    const DEFAULT_SCALE: f32 = 1.;
    pub fn run(
        vault_name: String,
        path: PathBuf,
        initial_file: Option<PathBuf>,
        config: Config
    ) -> Result<(), iced::Error> {
        iced::application(
            move || {
                (
                    Self {
                        config: config.clone(),
                        vault_name: vault_name.clone(),
                        vault_path: path.clone(),
                        window_manager: WindowManager::default(),
                        typed_command: None,
                        error: None,
                        error_id: 0,
                        command_history: CommandHistory::default(),
                        scale: Self::DEFAULT_SCALE
                    },
                    match initial_file.clone() {
                        Some(initial_file) => Task::done(Message::InitialFileOpen(initial_file)),
                        None => Task::none()
                    }
                )
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

    fn open_file(&mut self, path: PathBuf) -> Task<Message> {
        let (data, newly_created) = FILE_STORE.get_ref(path.clone());
        *self.window_manager.current_window_mut() = Window::Markdown(data);
        if newly_created {
            let absolute_path = self.vault_path.join(&path);
            return Task::future(async {
                let content = fs::read_to_string(absolute_path)
                    .await
                    .map_err(|error| error.to_string());
                Message::FileOpened { path, content }
            });
        }

        Task::none()
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
            Message::Type(TextInputId::CommandInput, command) => {
                self.command_history.deselect();
                self.typed_command = Some(command)
            }
            Message::FileOpened {
                path,
                content: Ok(source)
            } => FILE_STORE.insert(&path, MarkdownStore::new(source)),
            Message::FileOpened {
                path,
                content: Err(error)
            } => {
                return self
                    .display_error(format!("Cannot open {}: {error}", path.to_string_lossy()));
            }
            Message::Submit(TextInputId::CommandInput) => {
                if self.typed_command.as_ref().unwrap().is_empty() {
                    return Task::none();
                }

                self.command_history.deselect();
                let typed_command = self.typed_command.take().unwrap();
                let command: Command = match typed_command.parse() {
                    Ok(command) => command,
                    Err(error) => {
                        let error = format!("Error parsing command \"{typed_command}\": {error}");
                        return self.display_error(error);
                    }
                };

                self.command_history.push(typed_command);
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
            CommandKind::Open { path } => return self.open_file(path),
            CommandKind::Split { path } => {
                match path {
                    Some(path) => {
                        self.window_manager.add_window(Window::Empty);
                        return self.open_file(path);
                    }
                    None => self
                        .window_manager
                        .add_window(self.window_manager.current_window().clone())
                };
            }
            CommandKind::NextSplit => self.window_manager.next_window(),
            CommandKind::PreviousSplit => self.window_manager.previous_window(),
            CommandKind::ScaleUp => self.scale += scale.default_step,
            CommandKind::ScaleDown => self.scale -= scale.default_step,
            CommandKind::ScaleReset => self.scale = scale.default,
            CommandKind::HistoryUp => self.command_history.select_up(),
            CommandKind::HistoryDown => self.command_history.select_down(),
            CommandKind::CommandModeOpen => self.typed_command = Some(String::new()),
            CommandKind::Noop => {}
            CommandKind::CommandModeExit => self.typed_command = None
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let vault_name = Widget::container(
            self.vault_name.as_str(),
            ContainerKind::Aligned {
                horizontal: Some(Alignment::End),
                vertical: None
            }
        );

        let bar_content = match (
            self.command_history.currently_selected(),
            self.typed_command.as_deref(),
            &self.error
        ) {
            (Some(command), ..) | (_, Some(command), _) => row![
                Spacing::None,
                ":",
                Widget::TextInput {
                    content: command,
                    placeholder: "Enter command",
                    id: TextInputId::CommandInput
                },
                vault_name
            ],
            (_, _, Some(error)) => row![Spacing::None, Widget::Error(error.into()), vault_name],
            _ => vault_name
        };

        let bar = Widget::container(bar_content, ContainerKind::Bar);
        let windows = self.window_manager.render();
        column![Spacing::None, windows, bar].render(self.theme())
    }
}
