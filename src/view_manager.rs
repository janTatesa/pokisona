mod serde;
use std::{collections::HashMap, fs, io, ops::Deref, path::PathBuf, sync::LazyLock};

use ::serde::{Deserialize, Serialize};
use iced::widget::text_editor::{self};

use crate::{
    cache::{Cache, IdeaRef},
    markdown::Markdown,
    view_manager::serde::ViewSerde,
};

static PATH: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::data_dir().unwrap().join("pokisona/history.bin"));
pub struct ViewManager {
    view: View,
    pub picker: Option<Picker>,
    history: History,
}

impl Deref for ViewManager {
    type Target = View;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl Default for ViewManager {
    fn default() -> Self {
        Self {
            view: View::Title,
            picker: None,
            history: History {
                views: vec![ViewSerde::Title],
                index: 0,
            },
        }
    }
}

impl ViewManager {
    pub fn new(cache: &mut Cache) -> color_eyre::Result<Self> {
        Ok(if PATH.exists() {
            let history: History = postcard::from_bytes(&fs::read(&*PATH)?)?;
            Self {
                view: history.views[history.index].to_normal(cache)?,
                picker: None,
                history,
            }
        } else {
            Self::default()
        })
    }

    pub fn modify<T>(&mut self, f: impl FnOnce(&mut View) -> T) -> io::Result<T> {
        let out = f(&mut self.view);
        self.history.views[self.history.index] = self.view.to_serde();
        self.on_change()?;
        Ok(out)
    }

    fn on_change(&mut self) -> io::Result<()> {
        self.picker = None;
        fs::write(&*PATH, postcard::to_allocvec(&self.history).unwrap())
    }

    pub fn close_current(&mut self, cache: &mut Cache) -> io::Result<()> {
        if self.can_close_current() {
            self.history.views.remove(self.history.index);
            self.history.index -= 1;
            self.view = self.history.views[self.history.index].to_normal(cache)?;
            self.on_change()?;
        }

        Ok(())
    }

    pub fn can_close_current(&self) -> bool {
        self.history.views.len() != 1
    }

    pub fn forward(&mut self, cache: &mut Cache) -> io::Result<()> {
        if self.can_go_forward() {
            self.history.index += 1;
            self.view = self.history.views[self.history.index].to_normal(cache)?;
            self.on_change()?;
        }

        Ok(())
    }

    pub fn backward(&mut self, cache: &mut Cache) -> io::Result<()> {
        if self.can_go_backward() {
            self.history.index -= 1;
            self.view = self.history.views[self.history.index].to_normal(cache)?;
            self.on_change()?;
        }

        Ok(())
    }

    pub fn insert(&mut self, view: View) -> io::Result<()> {
        self.history.index += 1;
        if self.history.index < self.history.views.len() {
            self.history.views.drain(self.history.index..);
        }

        self.history.views.push(view.to_serde());
        self.view = view;
        self.on_change()
    }

    pub fn can_go_forward(&self) -> bool {
        (self.history.index + 1) < self.history.views.len()
    }

    pub fn can_go_backward(&self) -> bool {
        self.history.index != 0
    }
}

#[derive(Debug)]
pub enum View {
    Title,
    Editor(text_editor::Content),
    Idea {
        idea: IdeaRef,
        current_ideas: HashMap<IdeaRef, Markdown>,
    },
}

pub struct Picker {
    pub selected: Option<usize>,
    pub kind: PickerKind,
}

impl Picker {
    pub const MAX_IDEAS: usize = 5;
}

#[derive(Clone, Debug)]
pub enum PickerKind {
    Idea {
        query: String,
        ideas: Vec<(IdeaRef, Markdown)>,
        link: bool,
    },
    Tag {
        tags: Vec<String>,
    },
}

#[derive(Serialize, Deserialize)]
struct History {
    views: Vec<ViewSerde>,
    index: usize,
}
