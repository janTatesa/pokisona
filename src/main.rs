#![deny(clippy::disallowed_types)]
mod cli;
mod command;
mod error;
mod mode;

use std::{fs, mem};

use camino::{Utf8Path, Utf8PathBuf};
use catppuccin::PALETTE;
use iced::{
    Border, Font, Length, Theme, exit, padding,
    widget::{
        self, Button, Id, button, column, container,
        operation::focus,
        row,
        text::{self, Wrapping},
        text_editor::{self, Content}
    }
};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};

use crate::{
    cli::{InitialFile, VaultName},
    command::Command,
    error::{Error, Result},
    mode::Mode
};

struct Pokisona {
    vault_name: String,
    bottom_bar: BottomBar,
    file: Option<File>,
    scale: f32,
    editor_content: text_editor::Content,
    mode: Mode
}

#[derive(Clone)]
struct File {
    edited: bool,
    path: PathBuf
}

#[derive(Clone)]
enum BottomBar {
    Command(String),
    Error(Error),
    None
}

type Element<'a, M = Message> = iced::Element<'a, M>;
type Task<M = Message> = iced::Task<M>;

#[derive(Clone)]
enum Message {
    EnterCommandMode,
    ExitCommandMode,
    Command(Command),
    SwitchMode(Mode),
    EditCommand(String),
    SubmitCommand,
    EditorAction(text_editor::Action)
}

type PathBuf = Utf8PathBuf;
#[allow(dead_code)]
type Path = Utf8Path;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let (VaultName(vault_name), InitialFile(path)) = cli::handle_args()?;
    let (file, bottom_bar, content) = match path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(content) => (
                Some(File {
                    edited: false,
                    path
                }),
                BottomBar::None,
                text_editor::Content::with_text(&content)
            ),
            Err(error) => (
                None,
                BottomBar::Error(error.into()),
                text_editor::Content::new()
            )
        },
        None => (None, BottomBar::None, text_editor::Content::new())
    };

    iced::application(
        move || {
            let pokisona = Pokisona {
                vault_name: vault_name.clone(),
                file: file.clone(),
                mode: Mode::Normal,
                bottom_bar: bottom_bar.clone(),
                editor_content: content.clone(),
                scale: 1.0
            };

            (pokisona, focus("editor"))
        },
        Pokisona::update,
        Pokisona::view
    )
    .font(LUCIDE_FONT_BYTES)
    .theme(Pokisona::theme)
    .scale_factor(|app| app.scale)
    .run()?;
    Ok(())
}

const CATPPUCCIN_MOCHA: catppuccin::FlavorColors = PALETTE.mocha.colors;
impl Pokisona {
    fn update(&mut self, msg: Message) -> Task {
        let task = match self.try_update(msg) {
            Ok(task) => task,
            Err(error) => {
                self.bottom_bar = BottomBar::Error(error);
                focus(Id::new("editor"))
            }
        };

        let focus = match self.bottom_bar {
            BottomBar::Command(_) => focus("command-input"),
            _ => focus("editor")
        };

        Task::batch([task, focus])
    }

    fn try_update(&mut self, msg: Message) -> Result<Task> {
        match msg {
            Message::ExitCommandMode => self.bottom_bar = BottomBar::None,
            Message::SwitchMode(mode) => self.mode = mode,
            Message::EditCommand(command) => self.bottom_bar = BottomBar::Command(command),
            Message::SubmitCommand => {
                let BottomBar::Command(command) = &mut self.bottom_bar else {
                    return Ok(Task::none());
                };

                if command.is_empty() {
                    self.bottom_bar = BottomBar::None;
                    return Ok(Task::none());
                }

                let command = mem::take(command);
                self.bottom_bar = BottomBar::None;
                return self.handle_command(command.parse::<Command>()?);
            }
            Message::EditorAction(action) => {
                self.bottom_bar = BottomBar::None;
                if let text_editor::Action::Edit(_) = &action
                    && let Some(File { edited, .. }) = &mut self.file
                {
                    *edited = true;
                }

                self.editor_content.perform(action);
            }
            Message::Command(command) => return self.handle_command(command),
            Message::EnterCommandMode => {
                self.bottom_bar = BottomBar::Command(String::new());
            }
        };

        Ok(Task::none())
    }

    fn handle_command(&mut self, command: Command) -> Result<Task> {
        match command {
            Command::Quit => {
                if self.file.as_ref().is_none_or(|file| !file.edited) {
                    Ok(exit())
                } else {
                    Err(Error::CannotQuitWithUnsavedBuffer)
                }
            }
            Command::ForceQuit => Ok(exit()),
            Command::Write(path) => self.write(path, false, Task::none()),
            Command::ForceWrite(path) => self.write(path, true, Task::none()),
            Command::WriteQuit(path) => self.write(path, false, exit()),
            Command::ForceWriteQuit(path) => self.write(path, true, exit()),
            Command::Open(path) => {
                let exists = path.exists();
                self.file = Some(File {
                    edited: !exists,
                    path: path.clone()
                });

                self.editor_content = Content::new();
                if exists {
                    self.editor_content =
                        text_editor::Content::with_text(&fs::read_to_string(path)?);
                }

                Ok(Task::none())
            }
            Command::Reload => {
                if let Some(File { edited, path }) = &mut self.file {
                    self.editor_content =
                        text_editor::Content::with_text(&fs::read_to_string(&*path)?);
                    *edited = false;
                }

                Ok(Task::none())
            }
            Command::Remove => {
                if let Some(File { path, .. }) = &self.file {
                    fs::remove_file(path)?;
                    self.file = None;
                }

                Ok(Task::none())
            }
            Command::Move(new_path) => {
                if new_path.parent().is_some_and(|path| !path.exists()) {
                    return Err(Error::MoveParentDirectoryDoesntExist);
                }

                if let Some(File { path, .. }) = &self.file
                    && path.exists()
                {
                    fs::rename(path, &new_path)?
                }

                self.file = Some(File {
                    edited: self.file.as_ref().is_none_or(|file| file.edited),
                    path: new_path
                });

                Ok(Task::none())
            }
            Command::ForceMove(new_path) => {
                if let Some(parent) = new_path.parent()
                    && !parent.exists()
                {
                    fs::create_dir_all(parent)?;
                }

                if let Some(File { path, .. }) = &self.file
                    && path.exists()
                {
                    fs::rename(path, &new_path)?
                }

                self.file = Some(File {
                    edited: self.file.as_ref().is_none_or(|file| file.edited),
                    path: new_path
                });

                Ok(Task::none())
            }
        }
    }

    fn write(&mut self, path: Option<PathBuf>, force: bool, task: Task) -> Result<Task> {
        if let Some(path) = path {
            self.file = Some(File { edited: true, path });
        }

        let Some(File { edited, path }) = &mut self.file else {
            return Err(Error::NoPathSet);
        };

        if !*edited {
            return Ok(Task::none());
        }

        *edited = false;

        if let Some(parent) = path.parent()
            && parent != ""
            && !parent.exists()
        {
            if force {
                fs::create_dir(parent)?;
            } else {
                return Err(Error::WriteParentDirectoryDoesntExist);
            }
        }

        fs::write(path, self.editor_content.text())?;
        Ok(task)
    }

    const HIGHLIGHT_SCALE_ALPHA: f32 = 0.5;
    const TEXT_EDITOR_LINE_WIDTH: f32 = 700.0;
    const PADDING: f32 = 5.0;
    fn view(&self) -> Element<'_> {
        let editor = widget::text_editor(&self.editor_content)
            .style(|_, _| text_editor::Style {
                background: CATPPUCCIN_MOCHA.base.into(),
                border: Border::default(),
                placeholder: CATPPUCCIN_MOCHA.text.into(),
                value: CATPPUCCIN_MOCHA.text.into(),
                selection: self.mode.color().scale_alpha(Self::HIGHLIGHT_SCALE_ALPHA)
            })
            .id("editor")
            .highlight(
                self.file
                    .as_ref()
                    .and_then(|file| file.path.extension())
                    .unwrap_or("md"),
                iced_highlighter::Theme::Base16Mocha
            )
            .on_action(Message::EditorAction)
            .width(Self::TEXT_EDITOR_LINE_WIDTH)
            .wrapping(Wrapping::WordOrGlyph)
            .key_binding(self.mode.bindings())
            .padding(Self::PADDING)
            .height(Length::Fill);
        let editor = container(editor).center_x(Length::Fill);

        let mode = container(self.mode.as_ref())
            .style(|_| container::Style {
                text_color: Some(CATPPUCCIN_MOCHA.crust.into()),
                background: Some(self.mode.color().into()),
                ..Default::default()
            })
            .padding(padding::horizontal(Self::PADDING));

        let save_button = self.file.as_ref().filter(|file| file.edited).map(|_| {
            Self::button(Icon::Save)
                .style(|_, status| match status {
                    button::Status::Active => button::Style {
                        text_color: CATPPUCCIN_MOCHA.subtext0.into(),
                        ..Default::default()
                    },
                    button::Status::Hovered => button::Style {
                        text_color: self.mode.color(),
                        ..Default::default()
                    },
                    button::Status::Pressed => button::Style {
                        text_color: CATPPUCCIN_MOCHA.overlay0.into(),
                        ..Default::default()
                    },
                    button::Status::Disabled => todo!()
                })
                .on_press(Message::Command(Command::Write(None)))
                .padding(0)
        });

        let path = container(
            self.file
                .as_ref()
                .map(|file| widget::text(file.path.as_str()))
                .unwrap_or(widget::text("[scratch]").color(CATPPUCCIN_MOCHA.subtext0))
        )
        .center_x(Length::Fill);
        let bar_left =
            container(row![mode, save_button].spacing(Self::PADDING)).width(Length::Fill);
        let bar = container(row![
            bar_left,
            path,
            container(self.vault_name.as_str())
                .align_right(Length::Fill)
                .padding(padding::right(Self::PADDING))
        ])
        .style(|_| container::Style {
            background: Some(CATPPUCCIN_MOCHA.crust.into()),
            ..Default::default()
        });

        let bottom_bar: Option<Element<'_>> = match &self.bottom_bar {
            BottomBar::Command(command) => Some(
                sweeten::widget::text_input("Enter command", command)
                    .on_blur(Message::ExitCommandMode)
                    .on_input(Message::EditCommand)
                    .on_submit(Message::SubmitCommand)
                    .width(Length::Fill)
                    .id("command-input")
                    .icon(sweeten::widget::text_input::Icon {
                        font: Font::DEFAULT,
                        code_point: ':',
                        size: None,
                        spacing: 0.0,
                        side: sweeten::widget::text_input::Side::Left
                    })
                    .style(|theme: &Theme, _| sweeten::widget::text_input::Style {
                        background: theme.palette().background.into(),
                        border: Border::default(),
                        icon: self.mode.color(),
                        placeholder: self.theme().extended_palette().secondary.base.color,
                        value: self.theme().palette().text,
                        selection: self.mode.color().scale_alpha(Self::HIGHLIGHT_SCALE_ALPHA)
                    })
                    .padding(0)
                    .into()
            ),
            BottomBar::Error(error) => {
                Some(widget::text(error.to_string()).style(text::danger).into())
            }
            BottomBar::None => None
        };

        column![editor, bar, bottom_bar].into()
    }

    // TODO: use a custom theme implementation
    fn theme(&self) -> Theme {
        Theme::custom(
            "catppuccin-mocha-custom",
            iced::theme::Palette {
                background: CATPPUCCIN_MOCHA.base.into(),
                text: CATPPUCCIN_MOCHA.text.into(),
                primary: self.mode.color(),
                success: CATPPUCCIN_MOCHA.green.into(),
                warning: CATPPUCCIN_MOCHA.yellow.into(),
                danger: CATPPUCCIN_MOCHA.red.into()
            }
        )
    }

    fn button<'a>(icon: Icon) -> Button<'a, Message> {
        let content = widget::text(icon.unicode())
            .font(Font::with_name("lucide"))
            .shaping(text::Shaping::Advanced);
        button(content)
    }
}
