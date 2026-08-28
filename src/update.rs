use std::fs;

use iced::{
    clipboard,
    keyboard::{self, Key},
    widget::{operation::focus, text_editor::Content}
};
use log::error;
use norm::{
    Metric,
    fzf::{FzfParser, FzfV2}
};
use rand::seq::IndexedRandom;

use crate::{Message, Picker, Pokisona, Task, View, cache::IdeaRef, markdown::Markdown};

impl Pokisona {
    pub fn update(&mut self, msg: Message) -> Task {
        match self.try_update(msg) {
            Err(error) => {
                error!("{error}");
                self.error = Some(error.to_string());
                Task::none()
            }
            Ok(task) => task
        }
    }

    fn try_update(&mut self, msg: Message) -> anyhow::Result<Task> {
        self.error = None;

        match msg {
            Message::Refocus => {
                return Ok(if self.picker.is_some() {
                    focus("picker_query")
                } else {
                    focus("editor")
                });
            }
            Message::Editor(action) => {
                let View::NewIdea { content, .. } = &mut self.view else {
                    unreachable!()
                };

                content.perform(action);
            }
            Message::Save => {
                let View::NewIdea { content } = &self.view else {
                    unreachable!()
                };

                let contents = content.text();
                let markdown = Markdown::new(&contents);
                let idea = self.cache.create(&contents, &markdown)?;
                self.open_idea(idea, markdown)?;
            }

            Message::Open(idea) => {
                let contents = self.cache.read_idea(idea)?;
                self.open_idea(idea, Markdown::new(&contents))?;
            }
            Message::RevisitRandom => {
                let current = if let View::Revisiting { idea, .. } = &self.view {
                    Some(*idea)
                } else {
                    None
                };

                let files: Vec<_> = self
                    .cache
                    .iter()
                    .filter_map(|(idea, meta)| {
                        if Some(*idea) == current {
                            return None;
                        }

                        let duration = meta.last_accessed.elapsed().ok()?;
                        Some((*idea, duration))
                    })
                    .collect();

                if files.is_empty() {
                    return Ok(Task::none());
                }

                let idea = files
                    .choose_weighted(&mut rand::rng(), |(_, accessed)| accessed.as_secs())?
                    .0;
                return self.try_update(Message::Open(idea));
            }
            Message::OpenPicker => self.picker = Some(Picker::default()),
            Message::NewIdea => {
                self.view = View::NewIdea {
                    content: Content::new()
                };

                return Ok(focus("editor"));
            }
            Message::PickerQuery(new_query) => {
                let Some(Picker {
                    query,
                    top_entries,
                    selected
                }) = &mut self.picker
                else {
                    unreachable!()
                };

                *query = new_query;
                let mut fzf = FzfV2::new();
                let mut parser = FzfParser::new();
                let query = parser.parse(query);
                let mut weights: Vec<_> = self
                    .cache
                    .keys()
                    .filter_map(|idea| {
                        Some((
                            *idea,
                            fzf.distance(
                                query,
                                &fs::read_to_string(format!("{idea}.md")).unwrap()
                            )?
                        ))
                    })
                    .collect();
                weights.sort_by_key(|(_, distance)| *distance);
                *top_entries = weights
                    .into_iter()
                    .take(Picker::MAX_ENTRIES)
                    .map(|(idea, _)| {
                        (
                            idea,
                            Markdown::new(&fs::read_to_string(format!("{idea}.md")).unwrap())
                        )
                    })
                    .collect();
                *selected = (*selected).min(top_entries.len().checked_sub(1));
            }
            Message::PickerNext => {
                let Some(Picker {
                    selected,
                    top_entries,
                    ..
                }) = &mut self.picker
                else {
                    unreachable!()
                };

                *selected = Some(selected.map_or(0, |selected| selected + 1))
                    .min(top_entries.len().checked_sub(1));
            }
            Message::PickerPrevious => {
                let Some(Picker { selected, .. }) = &mut self.picker else {
                    unreachable!()
                };

                *selected = selected.and_then(|selected| selected.checked_sub(1));
            }
            Message::KeyPress(key, modifiers) => {
                return self.try_update(match (key.as_ref(), modifiers) {
                    (Key::Character("n"), keyboard::Modifiers::CTRL) => Message::NewIdea,
                    (Key::Character("f"), keyboard::Modifiers::CTRL) => Message::OpenPicker,
                    (Key::Character("r"), keyboard::Modifiers::CTRL) => Message::RevisitRandom,
                    (Key::Character("s"), keyboard::Modifiers::CTRL) => Message::Save,
                    (Key::Character("c"), keyboard::Modifiers::CTRL)
                        if let View::Revisiting { idea, .. } = self.view
                            && self.picker.is_none() =>
                    {
                        Message::CopyLink(idea)
                    }
                    (Key::Named(keyboard::key::Named::ArrowUp), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickerPrevious
                    }
                    (Key::Named(keyboard::key::Named::ArrowDown), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickerNext
                    }
                    (Key::Named(keyboard::key::Named::Escape), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::ClosePicker
                    }

                    _ => return Ok(Task::none())
                });
            }
            Message::ClosePicker => self.picker = None,
            Message::CopyLink(idea) => {
                return Ok(clipboard::write(format!("[[{idea}]]")));
            }
        }

        Ok(Task::none())
    }

    fn open_idea(&mut self, idea: IdeaRef, markdown: Markdown) -> anyhow::Result<()> {
        let mut links = Vec::new();
        for link in &self.cache[&idea].links {
            let content = fs::read_to_string(format!("{link}.md"))?;
            let markdown = Markdown::new(&content);
            links.push((*link, markdown));
        }

        let mut backlinks = Vec::new();
        for backlink in &self.cache[&idea].backlinks {
            let content = fs::read_to_string(format!("{backlink}.md"))?;
            let markdown = Markdown::new(&content);
            backlinks.push((*backlink, markdown));
        }

        self.view = View::Revisiting {
            idea,
            markdown,
            links,
            backlinks
        };
        self.picker = None;

        Ok(())
    }
}
