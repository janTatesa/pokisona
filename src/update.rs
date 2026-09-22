use std::fs;

use either::Either;
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

use crate::{
    Message, Picker, PickerKind, Pokisona, Task, View, cache::IdeaRef, markdown::Markdown
};

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
                return Ok(focus(if self.picker.is_some() {
                    "picker_query"
                } else {
                    "editor"
                }));
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

            Message::OpenIdea(idea) => {
                let contents = self.cache.read_idea(idea)?;
                self.open_idea(idea, Markdown::new(&contents))?;
            }
            Message::OpenRandom => {
                let current = if let View::Idea { idea, .. } = &self.view {
                    Some(*idea)
                } else {
                    None
                };

                let ideas = match &self.tag_filter {
                    Some(filter) => Either::Left(self.cache.tags()[filter].ideas.iter()),
                    None => Either::Right(self.cache.ideas().keys())
                };

                let files: Vec<_> = ideas
                    .into_iter()
                    .filter_map(|idea| {
                        if Some(*idea) == current {
                            return None;
                        }

                        let duration = self.cache.ideas()[idea].last_accessed.elapsed().ok()?;
                        Some((*idea, duration))
                    })
                    .collect();

                if files.is_empty() {
                    return Ok(Task::none());
                }

                let idea = files
                    .choose_weighted(&mut rand::rng(), |(_, accessed)| accessed.as_secs())?
                    .0;
                return self.try_update(Message::OpenIdea(idea));
            }

            Message::OpenIdeaPicker => {
                let ideas = self
                    .cache
                    .ideas()
                    .keys()
                    .rev()
                    .take(Picker::MAX_IDEAS)
                    .filter_map(|idea| {
                        let text = &fs::read_to_string(format!("{idea}.md"))
                            .inspect_err(|error| {
                                error!("Error while reading {idea}: {error}");
                            })
                            .ok()?;
                        Some((*idea, Markdown::new(text)))
                    })
                    .collect();
                self.picker = Some(Picker {
                    selected: None,
                    kind: PickerKind::Idea {
                        query: String::new(),
                        ideas
                    }
                });
                return Ok(focus("picker_query"));
            }
            Message::OpenTagPicker => {
                let mut tags: Vec<_> = self.cache.tags().keys().cloned().collect();
                tags.sort_by_key(|tag| self.cache.tags()[tag].ideas.last());
                self.picker = Some(Picker {
                    selected: None,
                    kind: PickerKind::Tag { tags }
                });
            }

            Message::NewIdea => {
                self.view = View::NewIdea {
                    content: Content::new()
                };

                return Ok(focus("editor"));
            }
            Message::PickerQuery(new_query) => {
                let Some(Picker {
                    selected,
                    kind: PickerKind::Idea { query, ideas }
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
                    .ideas()
                    .keys()
                    .filter_map(|idea| {
                        let candidate = &fs::read_to_string(format!("{idea}.md"))
                            .inspect_err(|error| {
                                error!("Error while reading {idea}: {error}");
                            })
                            .ok()?;
                        Some((idea, fzf.distance(query, candidate)?))
                    })
                    .collect();
                weights.sort_by_key(|(_, distance)| *distance);
                *ideas = weights
                    .into_iter()
                    .take(Picker::MAX_IDEAS)
                    .filter_map(|(idea, _)| {
                        let source = &fs::read_to_string(format!("{idea}.md"))
                            .inspect_err(|error| {
                                error!("Error while reading {idea}: {error}");
                            })
                            .ok()?;
                        Some((*idea, Markdown::new(source)))
                    })
                    .collect();
                *selected = (*selected).min(ideas.len().checked_sub(1));
            }
            Message::PickDown => {
                let picker = self.picker.as_mut().unwrap();
                let (len, increment) = match &picker.kind {
                    PickerKind::Idea { ideas, .. } => (ideas.len(), 1),
                    PickerKind::Tag { tags } => (tags.len(), 4)
                };

                picker.selected = Some(picker.selected.map_or(0, |selected| selected + increment))
                    .min(len.checked_sub(1));
            }
            Message::PickUp => {
                let picker = self.picker.as_mut().unwrap();
                picker.selected = picker.selected.and_then(|selected| selected.checked_sub(1));
            }

            Message::PickLeft => {
                if let Some(Picker {
                    kind: PickerKind::Tag { tags },
                    selected
                }) = &mut self.picker
                {
                    *selected = Some(selected.map_or(0, |selected| selected + 1))
                        .min(tags.len().checked_sub(1));
                }
            }
            Message::PickRight => {
                if let Some(Picker {
                    kind: PickerKind::Tag { .. },
                    selected
                }) = &mut self.picker
                {
                    *selected = selected.and_then(|selected| selected.checked_sub(1));
                }
            }

            Message::KeyPress(key, modifiers) => {
                return Ok(Task::done(match (key.as_ref(), modifiers) {
                    (Key::Character("n"), keyboard::Modifiers::CTRL) => Message::NewIdea,
                    (Key::Character("f"), keyboard::Modifiers::CTRL) => Message::OpenIdeaPicker,
                    (Key::Character("r"), keyboard::Modifiers::CTRL) => Message::OpenRandom,
                    (Key::Character("s"), keyboard::Modifiers::CTRL) => Message::Save,
                    (Key::Character("c"), keyboard::Modifiers::CTRL)
                        if let View::Idea { idea, .. } = self.view
                            && self.picker.is_none() =>
                    {
                        Message::CopyLink(idea)
                    }
                    (Key::Character("t"), keyboard::Modifiers::CTRL) => Message::OpenTagPicker,
                    (Key::Named(keyboard::key::Named::ArrowUp), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickUp
                    }
                    (Key::Named(keyboard::key::Named::ArrowDown), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickDown
                    }
                    (Key::Named(keyboard::key::Named::ArrowLeft), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickLeft
                    }
                    (Key::Named(keyboard::key::Named::ArrowDown), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::PickRight
                    }
                    (Key::Named(keyboard::key::Named::Escape), keyboard::Modifiers::NONE)
                        if self.picker.is_some() =>
                    {
                        Message::ClosePicker
                    }
                    (Key::Named(keyboard::key::Named::Escape), keyboard::Modifiers::NONE)
                        if self.tag_filter.is_some() =>
                    {
                        Message::UnsetTagFilter
                    }
                    (Key::Named(keyboard::key::Named::Enter), keyboard::Modifiers::NONE)
                        if let Some(Picker {
                            selected: Some(selected),
                            kind: PickerKind::Idea { ideas, .. }
                        }) = &self.picker =>
                    {
                        Message::OpenIdea(ideas[*selected].0)
                    }
                    (Key::Named(keyboard::key::Named::Enter), keyboard::Modifiers::NONE)
                        if let Some(Picker {
                            selected: Some(selected),
                            kind: PickerKind::Tag { tags }
                        }) = &self.picker =>
                    {
                        Message::SetTagFilter(tags[*selected].clone())
                    }

                    _ => return Ok(Task::none())
                }));
            }
            Message::ClosePicker => self.picker = None,
            Message::CopyLink(idea) => {
                return Ok(clipboard::write(format!("[[{idea}]]")));
            }
            Message::SetTagFilter(tag) => {
                self.picker = None;

                if let View::Idea { idea, .. } = &self.view
                    && !self.cache.ideas()[idea].tags.contains(&tag)
                {
                    self.view = View::None;
                }

                self.tag_filter = Some(tag);
            }
            Message::UnsetTagFilter => self.tag_filter = None
        }

        Ok(Task::none())
    }

    fn open_idea(&mut self, idea: IdeaRef, markdown: Markdown) -> anyhow::Result<()> {
        let mut links = Vec::new();
        for link in &self.cache.ideas()[&idea].links {
            let content = fs::read_to_string(format!("{link}.md"))?;
            let markdown = Markdown::new(&content);
            links.push((*link, markdown));
        }

        let mut backlinks = Vec::new();
        for backlink in &self.cache.ideas()[&idea].backlinks {
            let content = fs::read_to_string(format!("{backlink}.md"))?;
            let markdown = Markdown::new(&content);
            backlinks.push((*backlink, markdown));
        }

        self.view = View::Idea {
            idea,
            markdown,
            links,
            backlinks
        };
        self.picker = None;

        Ok(())
    }
}
