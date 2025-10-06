use std::path::PathBuf;

use color_eyre::Result;
use iced::{
    Alignment::Center,
    Background, Border, Color, Element, Event,
    Length::Fill,
    Task, Theme, event, exit,
    keyboard::{self, Key, Modifiers, key::Named},
    widget::{Id, Sensor, column, container, operation::focus, row, text, text_input}
};
use smol::fs;
use strum_macros::IntoStaticStr;

use crate::{
    color::MANTLE,
    command::{Command, CommandKind},
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
    FileOpened {
        path: PathBuf,
        content: String,
        split: bool
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

    fn update(&mut self, msg: Message) -> Task<Message> {
        dbg!(&msg);
        match msg {
            Message::TypeCommand(command) => self.typed_command = Some(command),
            Message::FileOpened {
                path,
                content,
                split: true
            } => self
                .window_manager
                .add_window(Window::Markdown(path, MarkdownStore::new(content))),
            Message::FileOpened {
                path,
                content,
                split: false
            } => self
                .window_manager
                .set_current_window(Window::Markdown(path, MarkdownStore::new(content))),
            Message::SubmitCommand => {
                let command: Command = self
                    .typed_command
                    .take()
                    .unwrap()
                    .parse()
                    .expect("Error handling not yet implemented");
                match command.kind {
                    CommandKind::Quit => _ = self.window_manager.remove_window(),
                    CommandKind::QuitAll => return exit(),
                    CommandKind::Open { path } => {
                        return Task::future(Self::read_file(self.vault_path.join(path), false));
                    }
                    CommandKind::Split { path } => {
                        match path {
                            Some(path) => {
                                return Task::future(Self::read_file(
                                    self.vault_path.join(path),
                                    true
                                ));
                            }
                            None => self.window_manager.add_window(Window::default())
                        };
                    }
                    CommandKind::NextSplit => self.window_manager.next_window(),
                    CommandKind::PreviousSplit => self.window_manager.previous_window()
                }
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

    async fn read_file(path: PathBuf, split: bool) -> Message {
        let content = fs::read_to_string(&path)
            .await
            .expect("Error handling not yet implemented");
        Message::FileOpened {
            path,
            content,
            split
        }
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
                .id(ElementId::CommandInput)
                .line_height(1.0);
            let sensored = Sensor::new(text_input).on_show(|_| Message::CommandInputSpawned);
            // TODO: The : doesn't align with the text
            let row = row![text(":").align_y(Center).line_height(1.0), sensored]
                .align_y(Center)
                .padding(5.0);
            container(row).style(|_| {
                container::Style::default()
                    .border(Border::default().rounded(5.0))
                    .background(Background::Color(MANTLE))
            })
        });

        container(column![self.window_manager.render(), command_input].spacing(5.0))
            .width(Fill)
            .height(Fill)
            .padding(5.0)
            .into()
    }
}
