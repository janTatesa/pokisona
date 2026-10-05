#![deny(clippy::pedantic)]
#![allow(clippy::unchecked_time_subtraction)]
#![allow(clippy::too_many_lines)]
#![deny(clippy::all)]

mod cache;
mod markdown;
mod theme;
mod update;
mod view;
mod view_manager;

use std::{cell::RefCell, env, fs};

use iced::{
    Event, Subscription, event,
    keyboard::{self, Key},
    widget::{
        operation::focus,
        text_editor::{self},
    },
};
use log::error;
use lucide_icons::LUCIDE_FONT_BYTES;

use crate::{
    cache::{Cache, IdeaRef},
    theme::CatppuccinFrappe,
    view_manager::ViewManager,
};

struct Pokisona {
    error: Option<String>,
    view_manager: ViewManager,
    cache: Cache,
    tag_filter: Option<String>,
}

#[derive(Clone, Debug)]
enum Message {
    NewIdea { content: text_editor::Content },
    Editor(text_editor::Action),

    Italic,
    Bold,
    Tag,
    AddLink(IdeaRef),
    List,
    NumberedList,
    Save,

    Refocus,

    KeyPress(Key, keyboard::Modifiers),

    SetTagFilter(String),
    UnsetTagFilter,

    OpenIdea(IdeaRef),
    OpenRandom,

    OpenIdeaPicker { link: bool },
    OpenTagPicker,
    PickerQuery(String),
    PickDown,
    PickUp,
    PickLeft,
    PickRight,
    ClosePicker,

    HistoryForward,
    HistoryBackward,
    HistoryClose,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    env_logger::try_init()?;

    let vault_path = dirs::data_dir().unwrap().join("pokisona/vault");

    fs::create_dir_all(&vault_path)?;
    env::set_current_dir(&vault_path)?;

    let mut cache = match Cache::new() {
        Ok(cache) => cache,
        Err(error) => {
            error!("Error while reading cache: {error}, recreating it");
            Cache::build()?
        }
    };

    let view_manager = ViewManager::new(&mut cache)
        .inspect_err(|error| error!("Error while reading history: {error}, recreating it"))
        .unwrap_or_default();
    let app = RefCell::new(Some(Pokisona {
        error: None,
        cache,
        view_manager,
        tag_filter: None,
    }));

    let boot = move || (app.borrow_mut().take().unwrap(), focus("editor"));
    iced::application(boot, Pokisona::update, Pokisona::view)
        .theme(Pokisona::theme)
        .settings(iced::Settings {
            id: Some("pokisona".to_string()),
            fonts: vec![
                LUCIDE_FONT_BYTES.into(),
                include_bytes!("../fonts/Libron_Regular.ttf").into(),
                include_bytes!("../fonts/Libron_Bold.ttf").into(),
                include_bytes!("../fonts/Libron_Italic.ttf").into(),
                include_bytes!("../fonts/Libron_BoldItalic.ttf").into(),
            ],
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
