use std::{
    fmt::{Display, Formatter},
    str::FromStr
};

use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Deserializer, de};

use crate::PathBuf;

#[derive(Parser)]
#[grammar = "./command.pest"]
struct CommandParser;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum CommandParseError {
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
            CommandParseError::TooManyArgs => "Too many arguments"
        };

        f.write_str(msg)
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pairs =
            CommandParser::parse(Rule::main, s).map_err(CommandParseError::InvalidSyntax)?;

        let command = match dbg!(pairs.next().unwrap().as_str()) {
            "quit" | "q" => Command::Quit,
            "quit-all" | "qa" => Command::QuitAll,
            "open" | "o" | "edit" | "e" => Command::Open {
                path: pairs
                    .next()
                    .ok_or(CommandParseError::NotEnoughArgs)?
                    .as_str()
                    .into()
            },
            "split" | "sp" => Command::Split {
                path: pairs.next().map(|pair| pair.as_str().into())
            },
            "vsplit" | "vs" => Command::VSplit {
                path: pairs.next().map(|pair| pair.as_str().into())
            },
            "hsplit" | "hs" => Command::HSplit {
                path: pairs.next().map(|pair| pair.as_str().into())
            },
            // "w" => Self::Write,
            // "wa" => Self::WriteAll,
            // "x" | "wq" => Self::WriteQuit,
            // "xa" | "wqa" => Self::WriteQuitAll,
            "next-window" => Command::NextWindow,
            "previous-window" => Command::PreviousWindow,
            "transpose-windows" => Command::TransposeWindows,
            "scale-up" => Command::ScaleUp,
            "scale-down" => Command::ScaleDown,
            "scale-reset" => Command::ScaleReset,
            "history-up" => Command::HistoryUp,
            "history-down" => Command::HistoryDown,
            "command-mode-open" => Command::CommandModeOpen,
            "noop" => Command::Noop,
            "command-mode-exit" => Command::CommandModeExit,
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

#[allow(clippy::enum_variant_names)]
#[derive(Clone)]
pub enum Command {
    Quit,
    QuitAll,
    // Write,
    // WriteAll,
    // WriteQuit,
    // WriteQuitAll,
    Open { path: PathBuf },

    Split { path: Option<PathBuf> },
    VSplit { path: Option<PathBuf> },
    HSplit { path: Option<PathBuf> },
    NextWindow,
    PreviousWindow,
    TransposeWindows,

    ScaleUp,
    ScaleDown,
    ScaleReset,

    HistoryUp,
    HistoryDown,
    CommandModeOpen,
    CommandModeExit,

    Noop
}
