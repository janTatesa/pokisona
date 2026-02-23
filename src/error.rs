use std::{fmt::Display, io::ErrorKind};

use crate::command::CommandParseErr;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub enum Error {
    Command(CommandParseErr),
    IO(ErrorKind),
    CannotQuitWithUnsavedBuffer,
    WriteParentDirectoryDoesntExist,
    MoveParentDirectoryDoesntExist,
    NoPathSet
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Command(command_parse_err) => match command_parse_err {
                CommandParseErr::TooManyArgs => f.write_str("Too many arguments"),
                CommandParseErr::NotEnoughArgs => f.write_str("Not enough arguments"),
                CommandParseErr::UnknownCommand => f.write_str("Unknown command")
            },
            Error::IO(error_kind) => write!(f, "IO error: {error_kind}"),
            Error::CannotQuitWithUnsavedBuffer => {
                f.write_str("Cannot quit with unsaved buffer, use q! to override")
            }
            Error::WriteParentDirectoryDoesntExist => {
                f.write_str("Parent directory doesn't exist, use w! to create")
            }
            Error::MoveParentDirectoryDoesntExist => {
                f.write_str("Parent directory doesn't exist, use mv! to create")
            }
            Error::NoPathSet => f.write_str("Cannot write with no path set")
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err.kind())
    }
}

impl From<CommandParseErr> for Error {
    fn from(err: CommandParseErr) -> Self {
        Self::Command(err)
    }
}
