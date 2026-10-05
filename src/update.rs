use std::{fs, io};

use either::Either;
use iced::{
    Task,
    keyboard::{self, Key, key},
    widget::{
        operation::focus,
        text_editor::{self, Content, Motion}
    }
};
use log::error;
use norm::{
    Metric,
    fzf::{FzfParser, FzfV2}
};
use rand::seq::IndexedRandom;

use crate::{
    Message, Pokisona,
    markdown::Markdown,
    view_manager::{Picker, PickerKind, View}
};

impl Pokisona {
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        self.error = None;

        match self.try_update(msg) {
            Err(error) => {
                error!("{error}");
                self.error = Some(error.to_string());
                Task::none()
            }
            Ok(task) => task
        }
    }

    fn try_update(&mut self, msg: Message) -> anyhow::Result<Task<Message>> {
        match msg {
            Message::Refocus => {
                return Ok(focus(if self.view_manager.picker.is_some() {
                    "picker_query"
                } else {
                    "editor"
                }));
            }

            Message::Editor(action) => {
                self.view_manager.modify(|view| {
                    let View::Editor(content) = view else {
                        unreachable!()
                    };

                    content.perform(action);
                })?;
            }
            Message::Save => {
                self.view_manager.modify(|view| -> io::Result<()> {
                    let View::Editor(content) = view else {
                        unreachable!()
                    };

                    let contents = content.text();
                    let markdown = Markdown::new(&contents);
                    let idea = self.cache.create(&contents, &markdown)?;
                    *view = View::Idea {
                        idea,
                        current_ideas: self.cache.read_relevant(idea)?
                    };

                    Ok(())
                })??;
            }

            Message::OpenIdea(idea) => self.view_manager.insert(View::Idea {
                idea,
                current_ideas: self.cache.read_relevant(idea)?
            })?,
            Message::OpenRandom => {
                let ideas = match &self.tag_filter {
                    Some(filter) => Either::Left(self.cache.tags()[filter].ideas.iter()),
                    None => Either::Right(self.cache.ideas().keys())
                };

                let files: Vec<_> = ideas
                    .into_iter()
                    .filter_map(|idea| {
                        if let View::Idea { idea: current, .. } = &*self.view_manager
                            && idea == current
                        {
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
                self.view_manager.insert(View::Idea {
                    idea,
                    current_ideas: self.cache.read_relevant(idea)?
                })?;
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
                let query = String::new();
                self.view_manager.picker = Some(Picker {
                    selected: None,
                    kind: PickerKind::Idea { query, ideas }
                });
                return Ok(focus("picker_query"));
            }
            Message::OpenTagPicker => {
                let mut tags: Vec<_> = self.cache.tags().keys().cloned().collect();
                tags.sort_by_key(|tag| self.cache.tags()[tag].ideas.last());
                tags.reverse();
                self.view_manager.picker = Some(Picker {
                    selected: None,
                    kind: PickerKind::Tag { tags }
                });
            }

            Message::NewIdea { content } => {
                self.view_manager.insert(View::Editor(content))?;
                return Ok(focus("editor"));
            }
            Message::PickerQuery(new_query) => {
                let Some(Picker {
                    selected,
                    kind: PickerKind::Idea { query, ideas }
                }) = &mut self.view_manager.picker
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
                let picker = self.view_manager.picker.as_mut().unwrap();
                let (len, increment) = match &picker.kind {
                    PickerKind::Idea { ideas, .. } => (ideas.len(), 1),
                    PickerKind::Tag { tags } => (tags.len(), 4)
                };

                picker.selected = Some(picker.selected.map_or(0, |selected| selected + increment))
                    .min(len.checked_sub(1));
            }
            Message::PickUp => {
                let picker = self.view_manager.picker.as_mut().unwrap();
                picker.selected = picker.selected.and_then(|selected| selected.checked_sub(1));
            }

            Message::PickRight => {
                if let Some(Picker {
                    kind: PickerKind::Tag { tags },
                    selected
                }) = &mut self.view_manager.picker
                {
                    *selected = Some(selected.map_or(0, |selected| selected + 1))
                        .min(tags.len().checked_sub(1));
                }
            }
            Message::PickLeft => {
                if let Some(Picker {
                    kind: PickerKind::Tag { .. },
                    selected
                }) = &mut self.view_manager.picker
                {
                    *selected = selected.and_then(|selected| selected.checked_sub(1));
                }
            }

            Message::KeyPress(key, modifiers) => {
                return Ok(Task::done(match (key.as_ref(), modifiers) {
                    (Key::Character("n"), keyboard::Modifiers::CTRL) => Message::NewIdea {
                        content: Content::new()
                    },
                    (Key::Character("f"), keyboard::Modifiers::CTRL) => Message::OpenIdeaPicker,
                    (Key::Character("r"), keyboard::Modifiers::CTRL) => Message::OpenRandom,
                    (Key::Character("s"), keyboard::Modifiers::CTRL) => Message::Save,
                    (Key::Character("t"), keyboard::Modifiers::CTRL) => Message::OpenTagPicker,
                    (Key::Named(key::Named::ArrowUp), keyboard::Modifiers::NONE)
                        if self.view_manager.picker.is_some() =>
                    {
                        Message::PickUp
                    }
                    (Key::Named(key::Named::ArrowDown), keyboard::Modifiers::NONE)
                        if self.view_manager.picker.is_some() =>
                    {
                        Message::PickDown
                    }
                    (Key::Named(key::Named::ArrowLeft), keyboard::Modifiers::NONE)
                        if self.view_manager.picker.is_some() =>
                    {
                        Message::PickLeft
                    }
                    (Key::Named(key::Named::ArrowRight), keyboard::Modifiers::NONE)
                        if self.view_manager.picker.is_some() =>
                    {
                        Message::PickRight
                    }
                    (Key::Named(key::Named::Escape), keyboard::Modifiers::NONE)
                        if self.view_manager.picker.is_some() =>
                    {
                        Message::ClosePicker
                    }
                    (Key::Named(key::Named::Escape), keyboard::Modifiers::NONE)
                        if self.tag_filter.is_some() =>
                    {
                        Message::UnsetTagFilter
                    }
                    (Key::Named(key::Named::Enter), keyboard::Modifiers::NONE)
                        if let Some(Picker {
                            selected: Some(selected),
                            kind: PickerKind::Idea { ideas, .. }
                        }) = &self.view_manager.picker =>
                    {
                        Message::OpenIdea(ideas[*selected].0)
                    }
                    (Key::Named(key::Named::Enter), keyboard::Modifiers::NONE)
                        if let Some(Picker {
                            selected: Some(selected),
                            kind: PickerKind::Tag { tags }
                        }) = &self.view_manager.picker =>
                    {
                        Message::SetTagFilter(tags[*selected].clone())
                    }
                    (Key::Named(key::Named::ArrowLeft), keyboard::Modifiers::ALT) => {
                        Message::HistoryBackward
                    }
                    (Key::Named(key::Named::ArrowRight), keyboard::Modifiers::ALT) => {
                        Message::HistoryForward
                    }
                    (Key::Character("p"), keyboard::Modifiers::CTRL)
                        if let View::Idea { idea, .. } = &*self.view_manager =>
                    {
                        Message::Reply(*idea)
                    }
                    (Key::Character("w"), keyboard::Modifiers::CTRL) => Message::HistoryClose,
                    _ => return Ok(Task::none())
                }));
            }
            Message::ClosePicker => self.view_manager.picker = None,
            Message::SetTagFilter(tag) => {
                self.tag_filter = Some(tag);
                return Ok(Task::done(Message::OpenRandom));
            }
            Message::UnsetTagFilter => self.tag_filter = None,
            Message::HistoryForward => {
                self.view_manager.forward(&mut self.cache)?;
                return Ok(Task::done(Message::Refocus));
            }
            Message::HistoryBackward => {
                self.view_manager.backward(&mut self.cache)?;
                return Ok(Task::done(Message::Refocus));
            }
            Message::HistoryClose => {
                self.view_manager.close_current(&mut self.cache)?;
                return Ok(Task::done(Message::Refocus));
            }
            Message::Reply(idea) => {
                let mut content = Content::with_text(&format!("[[{idea}]] "));
                content.perform(text_editor::Action::Move(Motion::End));
                return Ok(Task::done(Message::NewIdea { content }));
            }
        }

        Ok(Task::none())
    }
}
