use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr
};

use serde::{Deserialize, Deserializer, de};

#[derive(Clone)]
pub struct Command {
    pub _force: bool,
    pub kind: CommandKind
}

#[derive(Debug)]
pub enum CommandParseError {
    NotFound,
    NotEnoughArgs
}

impl Display for CommandParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            CommandParseError::NotFound => "Command not found",
            CommandParseError::NotEnoughArgs => "Not enough arguments"
        };

        f.write_str(msg)
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut args = s.split_whitespace();
        let mut name = args.next().ok_or(Self::Err::NotEnoughArgs)?;
        let _force = name.chars().next_back().ok_or(Self::Err::NotEnoughArgs)? == '!';
        if _force {
            name = &name[..(name.len() - 1)];
        }

        let kind = match name {
            "quit" | "q" => CommandKind::Quit,
            "quit-all" | "qa" => CommandKind::QuitAll,
            "open" | "o" | "edit" | "e" => CommandKind::Open {
                path: args.next().ok_or(CommandParseError::NotEnoughArgs)?.into()
            },
            "split" | "sp" => CommandKind::Split {
                path: args.next().map(PathBuf::from)
            },
            "vsplit" | "vs" => CommandKind::VSplit {
                path: args.next().map(PathBuf::from)
            },
            "hsplit" | "hs" => CommandKind::HSplit {
                path: args.next().map(PathBuf::from)
            },
            // "w" => Self::Write,
            // "wa" => Self::WriteAll,
            // "x" | "wq" => Self::WriteQuit,
            // "xa" | "wqa" => Self::WriteQuitAll,
            "next-window" => CommandKind::NextWindow,
            "previous-window" => CommandKind::PreviousWindow,
            "transpose-windows" => CommandKind::TransposeWindows,
            "scale-up" => CommandKind::ScaleUp,
            "scale-down" => CommandKind::ScaleDown,
            "scale-reset" => CommandKind::ScaleReset,
            "history-up" => CommandKind::HistoryUp,
            "history-down" => CommandKind::HistoryDown,
            "command-mode-open" => CommandKind::CommandModeOpen,
            "noop" => CommandKind::Noop,
            "command-mode-exit" => CommandKind::CommandModeExit,
            _ => return Err(CommandParseError::NotFound)
        };

        Ok(Self { _force, kind })
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

#[derive(Clone)]
pub enum CommandKind {
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
