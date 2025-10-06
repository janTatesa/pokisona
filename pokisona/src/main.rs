mod color;
mod command;
mod markdown_store;
mod window;

use std::{mem, path::PathBuf};

use clap::{ArgAction, Parser, Subcommand, command};
use color_eyre::{Result, eyre::OptionExt};
use iced::{
    Border, Element,
    Length::Fill,
    Task, Theme, exit,
    widget::{TextInput, column, container, text_input}
};
use pokisona_markdown::Markdown;
use smol::fs;

use crate::{
    color::ACCENT,
    command::{Command, CommandKind},
    markdown_store::MarkdownStore,
    window::{Window, WindowManager}
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<VaultCommand>
}

#[derive(Subcommand)]
enum VaultCommand {
    Open {
        name: String,
        #[arg(long, action = ArgAction::SetTrue)]
        set_default: bool
    },
    Delete {
        name: String
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut path = dirs::data_dir().ok_or_eyre("Cannot determine data dir")?;
    path.push("pokisona");

    let vault_name = match Cli::parse().subcommand {
        Some(VaultCommand::Open {
            name,
            set_default: true
        }) => {
            path.push("default");
            std::fs::write(&path, &name)?;
            path.pop();
            name
        }
        Some(VaultCommand::Open {
            name,
            set_default: false
        }) => name,
        Some(VaultCommand::Delete { name }) => {
            // TODO: create a confirmation prompt
            path.push(name);
            return Ok(std::fs::remove_dir(&path)?);
        }
        None => {
            path.push("default");
            let name = std::fs::read_to_string(&path)?;
            path.pop();
            name
        }
    };

    path.extend(["vaults", &vault_name]);

    std::fs::create_dir_all(&path)?;

    iced::application(
        move || Pokisona {
            vault_name: vault_name.clone(),
            vault_path: path.clone(),
            window_manager: WindowManager::default(),
            typed_command: String::new()
        },
        Pokisona::update,
        Pokisona::view
    )
    .theme(|_app: &Pokisona| Theme::CatppuccinMocha)
    .scale_factor(|_| 2.0)
    .run()?;
    Ok(())
}

struct Pokisona {
    vault_name: String,
    vault_path: PathBuf,
    window_manager: WindowManager,
    typed_command: String
}

#[derive(Debug, Clone)]
enum Message {
    TypeCommand(String),
    SubmitCommand,
    FileOpened {
        path: PathBuf,
        content: String,
        split: bool
    }
}

impl Pokisona {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::TypeCommand(command) => self.typed_command = command,
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
                let command: Command = mem::take(&mut self.typed_command)
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
        container(
            column![
                self.window_manager.render(),
                text_input("Enter command", &self.typed_command)
                    .on_input(Message::TypeCommand)
                    .on_submit(Message::SubmitCommand)
                    .style(|theme, status| {
                        let mut style = text_input::default(theme, status);
                        style.border = Border::default().rounded(5.0).width(2.5).color(ACCENT);
                        style
                    })
            ]
            .spacing(5.0)
        )
        .width(Fill)
        .height(Fill)
        .padding(5.0)
        .into()
    }
}
