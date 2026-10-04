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

use std::{cell::RefCell, env};

use iced::{
    Event, Subscription, event,
    keyboard::{self, Key},
    widget::{
        operation::focus,
        text_editor::{self}
    }
};
use log::error;
use lucide_icons::LUCIDE_FONT_BYTES;

use crate::{
    cache::{Cache, IdeaRef},
    theme::CatppuccinFrappe,
    view::BASE_FONT_SIZE,
    view_manager::ViewManager
};

struct Pokisona {
    error: Option<String>,
    view_manager: ViewManager,
    cache: Cache,
    tag_filter: Option<String>
}

type Element<'a, M = Message> = iced::Element<'a, M, CatppuccinFrappe>;
type Task<M = Message> = iced::Task<M>;

#[derive(Clone, Debug)]
enum Message {
    NewIdea { content: text_editor::Content },
    Editor(text_editor::Action),

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
    let mut cache = match Cache::load() {
        Ok(cache) => cache,
        Err(error) => {
            error!("Error while reading cache: {error}, recreating it");
            Cache::new()?
        }
    };

    let view_manager = ViewManager::new(&mut cache)
        .inspect_err(|error| error!("Error while reading history: {error}, recreating it"))
        .unwrap_or_default();
    let app = RefCell::new(Some(Pokisona {
        error: None,
        cache,
        view_manager,
        tag_filter: None
    }));

    let boot = move || (app.borrow_mut().take().unwrap(), focus("editor"));
    iced::application(boot, Pokisona::update, Pokisona::view)
        .settings(iced::Settings {
            default_text_size: BASE_FONT_SIZE.into(),
            ..Default::default()
        })
        .theme(Pokisona::theme)
        .font(LUCIDE_FONT_BYTES)
        .font(include_bytes!("../fonts/Libron_Regular.ttf"))
        .font(include_bytes!("../fonts/Libron_Bold.ttf"))
        .font(include_bytes!("../fonts/Libron_Italic.ttf"))
        .font(include_bytes!("../fonts/Libron_BoldItalic.ttf"))
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
