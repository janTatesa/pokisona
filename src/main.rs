#![deny(clippy::pedantic)]
#![allow(clippy::unchecked_time_subtraction)]
#![allow(clippy::too_many_lines)]
#![deny(clippy::all)]

mod cache;
mod history;
mod markdown;
mod theme;
mod update;
mod view;

use std::{cell::RefCell, env};

use iced::{
    Event, Subscription, event,
    keyboard::{self, Key},
    widget::text_editor::{self, Motion}
};
use log::warn;
use lucide_icons::LUCIDE_FONT_BYTES;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    cache::{Cache, IdeaRef},
    history::History,
    markdown::Markdown,
    theme::CatppuccinFrappe,
    view::BASE_FONT_SIZE
};

struct Pokisona {
    error: Option<String>,
    history: History,
    cache: Cache,
    tag_filter: Option<String>,
    picker: Option<Picker>
}

#[derive(Debug, Serialize, Deserialize)]
enum View {
    Title,
    NewIdea {
        #[serde(
            serialize_with = "serialize_content",
            deserialize_with = "deserialize_content"
        )]
        content: text_editor::Content
    },
    Idea {
        idea: IdeaRef,
        markdown: Markdown,
        links: Vec<(IdeaRef, Markdown)>,
        backlinks: Vec<(IdeaRef, Markdown)>
    }
}

fn serialize_content<S: Serializer>(
    content: &text_editor::Content,
    serializer: S
) -> Result<S::Ok, S::Error> {
    content.text().serialize(serializer)
}

fn deserialize_content<'a, D: Deserializer<'a>>(
    deserializer: D
) -> Result<text_editor::Content, D::Error> {
    let mut content = text_editor::Content::with_text(&String::deserialize(deserializer)?);
    content.perform(text_editor::Action::Move(Motion::DocumentEnd));
    Ok(content)
}

struct Picker {
    selected: Option<usize>,
    kind: PickerKind
}

impl Picker {
    const MAX_IDEAS: usize = 5;
}

#[derive(Clone, Debug)]
enum PickerKind {
    Idea {
        query: String,
        ideas: Vec<(IdeaRef, Markdown)>
    },
    Tag {
        tags: Vec<String>
    }
}

type Element<'a, M = Message> = iced::Element<'a, M, CatppuccinFrappe>;
type Task<M = Message> = iced::Task<M>;

#[derive(Clone, Debug)]
enum Message {
    NewIdea { content: text_editor::Content },
    Editor(text_editor::Action),
    CopyLink(IdeaRef),
    Reply(IdeaRef),
    Save,
    Refocus,

    KeyPress(Key, keyboard::Modifiers),

    SetTagFilter(String),
    UnsetTagFilter,

    OpenIdea(IdeaRef),
    OpenRandom,

    OpenIdeaPicker,
    OpenTagPicker,
    PickerQuery(String),
    PickDown,
    PickUp,
    PickLeft,
    PickRight,
    ClosePicker,

    HistoryForward,
    HistoryBackward,
    HistoryClose
}

fn main() -> anyhow::Result<()> {
    env_logger::try_init()?;

    let vault_path = dirs::data_dir().unwrap().join("pokisona/vault");

    env::set_current_dir(&vault_path)?;
    let cache = match Cache::load() {
        Ok(cache) => cache,
        Err(error) => {
            warn!("Error while reading cache: {error}, recreating it");
            Cache::new()?
        }
    };

    let history = match History::new() {
        Ok(history) => history,
        Err(error) => {
            warn!("Error while reading history: {error}, recreating it");
            History::default()
        }
    };

    let app = RefCell::new(Some(Pokisona {
        error: None,
        history,
        cache,
        picker: None,
        tag_filter: None
    }));

    let boot = move || app.borrow_mut().take().unwrap();
    iced::application(boot, Pokisona::update, Pokisona::view)
        .settings(iced::Settings {
            default_text_size: BASE_FONT_SIZE.into(),
            ..Default::default()
        })
        .theme(Pokisona::theme)
        .font(LUCIDE_FONT_BYTES)
        .font(include_bytes!("../Libron_Regular.ttf"))
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

            let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
                return None;
            };

            Some(Message::KeyPress(key, modifiers))
        })
    }

    #[expect(clippy::unused_self)]
    fn theme(&self) -> CatppuccinFrappe {
        CatppuccinFrappe
    }
}
