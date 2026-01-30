use std::{
    fmt::{Display, Formatter},
    str::FromStr
};

use iced::widget::pane_grid::{self, Direction, Pane, ResizeEvent, Target};
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, de};

use crate::{Link, Message, file_store::FileLocator};

#[derive(Parser)]
#[grammar = "./command.pest"]
struct CommandParser;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum CommandParseError {
    InvalidArg(String),
    NotFound,
    NotEnoughArgs,
    TooManyArgs,
    InvalidSyntax(pest::error::Error<Rule>)
}

impl Display for CommandParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            CommandParseError::NotFound => "Command not found",
            CommandParseError::NotEnoughArgs => "Not enough arguments",
            CommandParseError::InvalidSyntax(error) => &format!("Invalid syntax: {error}"),
            CommandParseError::TooManyArgs => "Too many arguments",
            CommandParseError::InvalidArg(arg) => &format!("Invalid argument: {arg}")
        };

        f.write_str(msg)
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs =
            CommandParser::parse(Rule::main, s).map_err(CommandParseError::InvalidSyntax)?;

        // TODO: parse more variants (maybe make a derive macro)
        let command = match pairs.next().unwrap().as_str() {
            "quit" | "q" => Command::Quit(None),
            "quit-all" | "qa" => Command::QuitAll,
            "open" | "o" | "edit" | "e" => Command::Open {
                locator: pairs
                    .next()
                    .ok_or(CommandParseError::NotEnoughArgs)?
                    .as_str()
                    .parse()
                    .unwrap()
            },
            "vsplit" | "vs" => Command::VSplit {
                locator: pairs.next().map(|pair| pair.as_str().parse().unwrap()),
                pane: None
            },
            "hsplit" | "hs" => Command::HSplit {
                locator: pairs.next().map(|pair| pair.as_str().parse().unwrap()),
                pane: None
            },
            // "w" => Self::Write,
            // "wa" => Self::WriteAll,
            // "x" | "wq" => Self::WriteQuit,
            // "xa" | "wqa" => Self::WriteQuitAll,
            "scale-up" => Command::ScaleUp,
            "scale-down" => Command::ScaleDown,
            "scale-reset" => Command::ScaleReset,
            "history-up" => Command::HistoryUp,
            "history-down" => Command::HistoryDown,
            "command-mode-open" => Command::CommandModeOpen,
            "noop" => Command::Noop,
            "command-mode-exit" => Command::CommandModeExit,
            "file-history-forward" => Command::FileHistoryForward,
            "file-history-backward" => Command::FileHistoryBackward,
            "focus-adjacent" => Command::FocusAdjacent(
                match pairs
                    .next()
                    .ok_or(CommandParseError::NotEnoughArgs)?
                    .as_str()
                {
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    "left" => Direction::Left,
                    "right" => Direction::Right,
                    arg => return Err(CommandParseError::InvalidArg(arg.to_string()))
                }
            ),
            _ => return Err(CommandParseError::NotFound)
        };

        if pairs.next().is_some() {
            return Err(CommandParseError::TooManyArgs);
        }

        Ok(command)
    }
}

impl<'a> Deserialize<'a> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>
    {
        let string = String::deserialize(deserializer)?;
        Self::from_str(&string).map_err(de::Error::custom)
    }
}

/// Unlike [`Message`](crate::app::Message) [`Command`] can be produced by the user (and in future plugins). Generally anything that's not the result of a [`Future`] or which only makes sense with a mouse interaction (such as [`Message::HoverEnd`](crate::app::Message::HoverEnd)) should be a [`Command`] to maximise keyboard centricism and plugin capabilities. However currently the usser cannot construct all possible instances of command due to [`Pane`] having a private field. This might be fixed by either implementing our own [`pane_grid`] (which we might eventually do anyways) or generating our own pane ids
// TODO: have commands syntax in it's definition
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
pub(crate) enum Command {
    /// When [`None`] will quit the focused [`Pane`]
    Quit(Option<Pane>),
    QuitAll,
    // Write,
    // WriteAll,
    // WriteQuit,
    // WriteQuitAll,
    Open {
        locator: FileLocator
    },
    VSplit {
        locator: Option<FileLocator>,
        pane: Option<Pane>
    },
    HSplit {
        locator: Option<FileLocator>,
        pane: Option<Pane>
    },
    FocusPane(Pane),
    DropPane {
        pane: Pane,
        target: Target
    },
    ResizePane(ResizeEvent),

    FocusAdjacent(pane_grid::Direction),

    Error(String),

    Follow(Link),

    ScaleUp,
    ScaleDown,
    ScaleReset,

    CommandLineSet(String),
    CommandLineSubmit,
    HistoryUp,
    HistoryDown,
    CommandModeOpen,
    CommandModeExit,

    FileHistoryForward,
    FileHistoryBackward,

    Noop
}

impl From<Command> for Message {
    fn from(value: Command) -> Self {
        Self::Command(value)
    }
}
