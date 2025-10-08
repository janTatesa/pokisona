use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr
};

use iced::widget::text::Highlighter;

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

        let mut kind = CommandKind::from_str(name)
            .ok()
            .or(CommandKind::from_alias(name))
            .ok_or(Self::Err::NotFound)?;
        let arg = args.next();
        if let CommandKind::Open { path } = &mut kind {
            *path = PathBuf::from_str(arg.ok_or(Self::Err::NotEnoughArgs)?).unwrap();
        }

        if let CommandKind::Split { path } = &mut kind
            && let Some(arg) = arg
        {
            *path = PathBuf::from_str(arg).ok();
        }

        Ok(Self { _force, kind })
    }
}

#[derive(strum_macros::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum CommandKind {
    Quit,
    QuitAll,
    // Write,
    // WriteAll,
    // WriteQuit,
    // WriteQuitAll,
    Open { path: PathBuf },
    Split { path: Option<PathBuf> },
    NextSplit,
    PreviousSplit
}

impl CommandKind {
    fn from_alias(alias: &str) -> Option<Self> {
        Some(match alias {
            "q" => Self::Quit,
            "qa" => Self::QuitAll,
            "e" | "edit" | "o" => Self::Open {
                path: PathBuf::new()
            },
            "sp" => Self::Split { path: None },
            // "w" => Self::Write,
            // "wa" => Self::WriteAll,
            // "x" | "wq" => Self::WriteQuit,
            // "xa" | "wqa" => Self::WriteQuitAll,
            _ => return None
        })
    }
}
