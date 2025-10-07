use std::path::PathBuf;

use color_eyre::Result;
use iced::{
    Background, Border, Color, Element, Event,
    Length::Fill,
    Task, Theme, event, exit,
    keyboard::{self, Key, Modifiers, key::Named},
    widget::{Id, Sensor, column, container, operation::focus, stack, text_input}
};
use smol::fs;
use strum_macros::IntoStaticStr;

use crate::{
    color::CRUST,
    command::{Command, CommandKind},
    file_store::FILE_STORE,
    markdown_store::MarkdownStore,
    window::{Window, WindowManager}
};

pub struct Pokisona {
    _vault_name: String,
    vault_path: PathBuf,
    window_manager: WindowManager,
    typed_command: Option<String>
}

#[derive(Debug, Clone)]
pub enum Message {
    TypeCommand(String),
    SubmitCommand,
    CommandInputSpawned,
    UncapturedIcedEvent(Event),
    FileOpened { path: PathBuf, content: String }
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
                _vault_name: vault_name.clone(),
                vault_path: path.clone(),
                window_manager: WindowManager::default(),
                typed_command: None
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
                    .expect("Error handling not yet implemented");
                Message::FileOpened { path, content }
            });
        }

        Task::none()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::TypeCommand(command) => self.typed_command = Some(command),
            Message::FileOpened { path, content } => {
                FILE_STORE.insert(&path, MarkdownStore::new(content))
            }
            Message::SubmitCommand => {
                let command: Command = self
                    .typed_command
                    .take()
                    .unwrap()
                    .parse()
                    .expect("Error handling not yet implemented");
                return self.handle_command(command);
            }
            Message::CommandInputSpawned => return focus(ElementId::CommandInput),
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: Key::Character(char),
                modifiers,
                ..
            })) if char == ":" && !modifiers.contains(Modifiers::CTRL | Modifiers::ALT) => {
                self.typed_command = Some(String::new())
            }
            Message::UncapturedIcedEvent(Event::Keyboard(keyboard::Event::KeyReleased {
                modified_key: Key::Named(Named::Escape),
                modifiers,
                ..
            })) if modifiers == Modifiers::empty() => self.typed_command = None,
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
        let command_input = self.typed_command.as_ref().map(|command| {
            let text_input = text_input("Enter command", command)
                .on_input(Message::TypeCommand)
                .on_submit(Message::SubmitCommand)
                .style(|theme, status| {
                    let mut style = text_input::default(theme, status);
                    style.border = Border::default();
                    style.background = Background::Color(Color::TRANSPARENT);
                    style
                })
                .id(ElementId::CommandInput);
            let sensored = Sensor::new(text_input).on_show(|_| Message::CommandInputSpawned);
            container(stack![sensored, container(":").center_y(Fill)])
                .style(|_| container::background(CRUST))
        });

        column![
            container(self.window_manager.render())
                .width(Fill)
                .height(Fill)
                .padding(5.0),
            command_input
        ]
        .into()
    }
}
