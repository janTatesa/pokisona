use std::{path::PathBuf, time::Duration};

use color_eyre::Result;
use iced::{
    Alignment::End,
    Background, Border, Color, Element, Event,
    Length::Fill,
    Task, Theme, event, exit,
    keyboard::{self, Key, Modifiers, key::Named},
    widget::{Id, Sensor, column, container, operation::focus, row, text, text_input}
};
use smol::{Timer, fs};
use strum_macros::IntoStaticStr;

use crate::{
    color::{ACCENT, CRUST},
    command::{Command, CommandKind},
    command_history::CommandHistory,
    file_store::FILE_STORE,
    markdown_store::MarkdownStore,
    window::{Window, WindowManager}
};

pub struct Pokisona {
    vault_name: String,
    vault_path: PathBuf,
    window_manager: WindowManager,

    command_history: CommandHistory,
    typed_command: Option<String>,

    error_id: u64,
    error: Option<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    TypeCommand(String),
    SubmitCommand,
    CommandInputSpawned,
    UncapturedIcedEvent(Event),
    ClearError(u64),
    FileOpened {
        path: PathBuf,
        // HACK: We have to use string because message has to be clone
        content: Result<String, String>
    }
}

impl Message {
    fn from_iced_event(event: Event, status: event::Status, _: iced::window::Id) -> Option<Self> {
        match status {
            event::Status::Ignored => Some(Self::UncapturedIcedEvent(event)),
            event::Status::Captured => None
        }
    }
}

#[derive(IntoStaticStr)]
enum ElementId {
    CommandInput
}

impl From<ElementId> for Id {
    fn from(val: ElementId) -> Self {
        Self::new(val.into())
    }
}

impl Pokisona {
    pub fn run(vault_name: String, path: PathBuf) -> Result<(), iced::Error> {
        iced::application(
            move || Self {
                vault_name: vault_name.clone(),
                vault_path: path.clone(),
                window_manager: WindowManager::default(),
                typed_command: None,
                error: None,
                error_id: 0,
                command_history: CommandHistory::default()
            },
            Self::update,
            Self::view
        )
        .theme(Self::theme)
        .subscription(|_| event::listen_with(Message::from_iced_event))
        .scale_factor(|_| 2.0)
        .run()
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
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
            Message::TypeCommand(command) => {
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
            Message::SubmitCommand => {
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
            Message::CommandInputSpawned => return focus(ElementId::CommandInput),
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyReleased {
                modified_key: Key::Character(char),
                modifiers,
                ..
            })) if char == ":"
                && !modifiers.contains(Modifiers::CTRL | Modifiers::ALT)
                && self.typed_command.is_none() =>
            {
                self.typed_command = Some(String::new())
            }
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyReleased {
                modified_key: Key::Named(Named::Escape),
                modifiers,
                ..
            })) if modifiers == Modifiers::empty() => {
                self.command_history.deselect();
                self.typed_command = None
            }
            Message::ClearError(id) if self.error_id == id => self.error = None,
            Message::ClearError(_) => {}
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyReleased {
                key: Key::Named(Named::ArrowUp),
                modifiers,
                ..
            })) if modifiers == Modifiers::empty() => self.command_history.select_up(),
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyReleased {
                key: Key::Named(Named::ArrowDown),
                modifiers,
                ..
            })) if modifiers == Modifiers::empty() => self.command_history.select_down(),
            Message::UncapturedIcedEvent(_) => {}
        }

        Task::none()
    }

    // TODO: Stupid app has for some reason a big break on command submit
    fn handle_command(&mut self, command: Command) -> Task<Message> {
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
            CommandKind::PreviousSplit => self.window_manager.previous_window()
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let vault_name = container(self.vault_name.as_str())
            .width(Fill)
            .align_x(End)
            .into();

        let bar_content: Element<'_, Message> = match (
            self.command_history.currently_selected(),
            self.typed_command.as_deref(),
            &self.error
        ) {
            (Some(command), ..) | (_, Some(command), _) => {
                let text_input = text_input("Enter command", command)
                    .on_input(Message::TypeCommand)
                    .on_submit(Message::SubmitCommand)
                    .style(|theme, status| {
                        let mut style = text_input::default(theme, status);
                        style.border = Border::default();
                        style.background = Background::Color(Color::TRANSPARENT);
                        style
                    })
                    .id(ElementId::CommandInput)
                    .padding(0);
                let sensored = Sensor::new(text_input).on_show(|_| Message::CommandInputSpawned);
                row![":", sensored, vault_name].into()
            }
            (_, _, Some(error)) => row![text(error).style(text::danger), vault_name].into(),
            _ => vault_name
        };

        let bar = container(bar_content)
            .style(|_| container::background(CRUST))
            .width(Fill);
        let windows = container(self.window_manager.render())
            .width(Fill)
            .height(Fill)
            .padding(5.0);
        column![windows, bar].into()
    }
}
