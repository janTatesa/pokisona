#![deny(clippy::pedantic)]
#![allow(clippy::unchecked_time_subtraction)]
#![allow(clippy::too_many_lines)]
#![deny(clippy::all)]

mod cache;
mod markdown;
mod update;
mod view;

use std::{
    cell::RefCell,
    env,
    fs::{self},
    path::PathBuf
};

use anyhow::anyhow;
use clap::Parser;
use iced::{
    Event, Subscription, Theme, event,
    keyboard::{self, Key},
    widget::text_editor::{self}
};
use log::warn;
use lucide_icons::LUCIDE_FONT_BYTES;

use crate::{
    cache::{Cache, IdeaRef},
    markdown::Markdown
};

struct Pokisona {
    error: Option<String>,
    view: View,
    cache: Cache,
    picker: Option<Picker>
}

#[derive(Debug)]
enum View {
    Title(Markdown),
    NewIdea {
        content: text_editor::Content
    },
    Revisiting {
        idea: IdeaRef,
        markdown: Markdown,
        links: Vec<(IdeaRef, Markdown)>,
        backlinks: Vec<(IdeaRef, Markdown)>
    } // GraphView
}

impl Default for View {
    fn default() -> Self {
        Self::Title(Markdown::new(include_str!("../README.md")))
    }
}

#[derive(Default)]
struct Picker {
    query: String,
    top_entries: Vec<(IdeaRef, Markdown)>,
    selected: Option<usize>
}

impl Picker {
    const MAX_ENTRIES: usize = 5;
}

type Element<'a, M = Message> = iced::Element<'a, M>;
type Task<M = Message> = iced::Task<M>;

#[derive(Clone, Debug)]
enum Message {
    NewIdea,
    Editor(text_editor::Action),
    CopyLink(IdeaRef),
    Save,
    Refocus,

    KeyPress(Key, keyboard::Modifiers),

    Open(IdeaRef),
    RevisitRandom,

    OpenPicker,
    PickerQuery(String),
    PickerNext,
    ClosePicker,
    PickerPrevious
}

#[derive(Parser)]
#[command(version)]
struct Cli {
    vault_path: Option<PathBuf>
}

fn main() -> anyhow::Result<()> {
    env_logger::try_init()?;

    let cli = Cli::parse();
    let last_vault_file_path = dirs::state_dir()
        .or(dirs::data_dir())
        .expect("Should work on all platforms")
        .join("pokisona/last_vault");
    let vault_path = if let Some(path) = cli.vault_path {
        let path = if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(path)
        };

        fs::create_dir_all(last_vault_file_path.parent().unwrap())?;
        fs::write(last_vault_file_path, path.to_str().unwrap())?;
        path
    } else if !last_vault_file_path.exists() {
        return Err(anyhow!("You have to specify vault path on first startup"));
    } else {
        fs::read_to_string(last_vault_file_path)?.into()
    };

    fs::create_dir_all(format!("{}/.pokisona", vault_path.to_str().unwrap()))?;
    env::set_current_dir(&vault_path)?;

    let cache = match Cache::load() {
        Ok(cache) => cache,
        Err(error) => {
            warn!("Error while reading cache: {error}, recreating it");
            Cache::new()?
        }
    };
    let cache_wrapped = RefCell::new(Some(cache));
    let boot = move || Pokisona {
        error: None,
        view: View::default(),
        cache: cache_wrapped.take().unwrap(),
        picker: None
    };

    iced::application(boot, Pokisona::update, Pokisona::view)
        .font(LUCIDE_FONT_BYTES)
        .settings(iced::Settings {
            default_text_size: Pokisona::BASE_FONT_SIZE.into(),
            ..Default::default()
        })
        .subscription(Pokisona::subscription)
        .run()?;
    Ok(())
}

impl Pokisona {
    #[expect(clippy::unused_self)]
    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, _, _| {
            if let Event::Mouse(_) = event {
                return Some(Message::Refocus);
            }

            let Event::Keyboard(event) = event else {
                return None;
            };

            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };

            Some(Message::KeyPress(key, modifiers))
        })
    }

    #[expect(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::CatppuccinFrappe
    }
}
