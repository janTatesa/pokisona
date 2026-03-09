use std::str::FromStr;

use chumsky::{prelude::*, text::whitespace};

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

fn escape<'a>() -> impl Parser<'a, &'a str, char> {
    just('\\').ignore_then(any()).boxed()
}

fn ident<'a>() -> impl Parser<'a, &'a str, &'a str> {
    any()
        .filter(|c: &char| c.is_ascii_alphabetic() || *c == '-')
        .repeated()
        .at_least(1)
        .to_slice()
}

// TODO: maybe this could be simplified
fn args_parser<'a>() -> impl Parser<'a, &'a str, (&'a str, Vec<String>)> {
    ident()
        .then(
            whitespace()
                .ignore_then(choice((
                    none_of("\\\"")
                        .or(escape())
                        .repeated()
                        .collect()
                        .delimited_by(just('"'), just('"'))
                        .boxed(),
                    none_of("\\\'")
                        .or(escape())
                        .repeated()
                        .collect()
                        .delimited_by(just('\''), just('\''))
                        .boxed(),
                    any()
                        .filter(|c: &char| !c.is_ascii_whitespace())
                        .repeated()
                        .at_least(1)
                        .to_slice()
                        .map(ToString::to_string)
                        .boxed()
                )))
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>()
                .then_ignore(whitespace())
                .boxed()
        )
        .or(ident()
            .then_ignore(whitespace())
            .map(|ident| (ident, vec![])))
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::command::args_parser;

    #[test]
    fn test_parse_command() {
        let parsed = args_parser().parse("foo-bar 'baz\\'' baz  ");
        dbg!(parsed.errors().collect::<Vec<_>>());
        assert_eq!(
            parsed.into_output().unwrap(),
            ("foo-bar", vec!["baz'".to_string(), "baz".to_string()])
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CommandParseErr {
    TooManyArgs,
    NotEnoughArgs,
    CannotParse,
    Unknown
}

impl FromStr for Command {
    type Err = CommandParseErr;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let (name, args) = args_parser()
            .parse(str)
            .into_output()
            .ok_or(CommandParseErr::CannotParse)?;
        let mut args = args.into_iter();
        let command = match name {
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
            _ => return Err(CommandParseErr::Unknown)
        };

        if args.next().is_some() {
            return Err(CommandParseErr::TooManyArgs);
        }

        Ok(command)
    }
}
