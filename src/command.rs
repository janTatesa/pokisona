use std::{
    iter::Enumerate,
    str::{Chars, FromStr}
};

use crate::PathBuf;

#[derive(Clone)]
pub enum Command {
    Quit,
    ForceQuit,
    Write(Option<PathBuf>),
    ForceWrite(Option<PathBuf>),
    WriteQuit(Option<PathBuf>),
    ForceWriteQuit(Option<PathBuf>),
    Reload,
    Remove,
    Open(PathBuf),
    Move(PathBuf),
    ForceMove(PathBuf)
}

struct CommandArgsParser<'a> {
    str: &'a str,
    inner: Enumerate<Chars<'a>>
}

// TODO: use chumsky
impl<'a> Iterator for CommandArgsParser<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (mut start_idx, char) = self.inner.next()?;
        let inner = &mut self.inner;
        let end_idx = match char {
            '\'' if start_idx != 0
                && self.str.len() > start_idx + 1
                && self.str[(start_idx + 1)..].contains('\'') =>
            {
                start_idx += 1;
                inner
                    .take_while(|(_, char)| *char != '\'' && !char.is_whitespace())
                    .last()?
                    .0
            }
            '"' if start_idx != 0
                && self.str.len() > start_idx + 1
                && self.str[(start_idx + 1)..].contains('"') =>
            {
                start_idx += 1;
                inner
                    .take_while(|(_, char)| *char != '"' && !char.is_whitespace())
                    .last()?
                    .0
            }
            _ => inner
                .take_while(|(_, char)| !char.is_whitespace())
                .last()
                .map(|(num, _)| num)
                .unwrap_or(start_idx)
        };

        Some(&self.str[start_idx..=end_idx])
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CommandParseErr {
    TooManyArgs,
    NotEnoughArgs,
    UnknownCommand
}

impl FromStr for Command {
    type Err = CommandParseErr;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let mut args = CommandArgsParser {
            str,
            inner: str.chars().enumerate()
        };
        let command_name = args.next().ok_or(CommandParseErr::NotEnoughArgs)?;
        let command = match command_name {
            "q" | "quit" => Command::Quit,
            "q!" | "quit!" => Command::ForceQuit,
            "w" | "write" => Command::Write(args.next().map(PathBuf::from)),
            "w!" | "write!" => Command::ForceWrite(args.next().map(PathBuf::from)),
            "wq" | "x" | "write-quit" => Command::WriteQuit(args.next().map(PathBuf::from)),
            "wq!" | "x!" | "write-quit!" => Command::ForceWriteQuit(args.next().map(PathBuf::from)),
            "o" | "open" => {
                Command::Open(args.next().ok_or(CommandParseErr::NotEnoughArgs)?.into())
            }
            "rl" | "reload" => Command::Reload,
            "rm" | "remove" => Command::Remove,
            "mv" | "move" => {
                Command::Move(args.next().ok_or(CommandParseErr::NotEnoughArgs)?.into())
            }
            "mv!" | "move!" => {
                Command::ForceMove(args.next().ok_or(CommandParseErr::NotEnoughArgs)?.into())
            }
            _ => return Err(CommandParseErr::UnknownCommand)
        };

        if args.next().is_some() {
            return Err(CommandParseErr::TooManyArgs);
        }

        Ok(command)
    }
}
