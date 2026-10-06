use std::{
    cmp::{max, min},
    fs, io,
    sync::Arc,
};

use chumsky::Parser;
use either::Either;
use iced::{
    Task,
    keyboard::{self, Key, key},
    widget::{
        operation::focus,
        text::Position,
        text_editor::{self, Content, Cursor, Edit},
    },
};
use log::error;
use norm::{
    Metric,
    fzf::{FzfParser, FzfV2},
};
use rand::seq::IndexedRandom;

use crate::{
    Message, Pokisona,
    markdown::{ListItem, Markdown, MarkdownLine},
    view_manager::{Picker, PickerKind, View},
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
            Ok(task) => task,
        }
    }

    fn try_update(&mut self, msg: Message) -> color_eyre::Result<Task<Message>> {
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
                        panic!()
                    };

                    if let text_editor::Action::Edit(Edit::Enter) = &action {
                        let line = content.line(content.cursor().position.line).unwrap();
                        let line = MarkdownLine::parser().parse(&line.text).unwrap();
                        let start = match line.list_item {
                            Some((_, ListItem::Bullet)) => "- ".to_string(),
                            Some((_, ListItem::Number(number))) => format!("{}. ", number + 1),
                            None => String::new(),
                        };
                        content.perform(action);
                        content.perform(text_editor::Action::Edit(Edit::Paste(Arc::new(start))));
                    } else {
                        content.perform(action);
                    }
                })?;
            }
            Message::Save => {
                self.view_manager.modify(|view| -> io::Result<()> {
                    let View::Editor(content) = view else {
                        panic!()
                    };

                    let contents = content.text();
                    let markdown = Markdown::new(&contents);
                    let idea = self.cache.create(&contents, &markdown)?;
                    *view = View::Idea {
                        idea,
                        current_ideas: self.cache.read_relevant(idea)?,
                    };

                    Ok(())
                })??;
            }

            Message::OpenIdea(idea) => self.view_manager.insert(View::Idea {
                idea,
                current_ideas: self.cache.read_relevant(idea)?,
            })?,
            Message::OpenRandom => {
                let ideas = match &self.tag_filter {
                    Some(filter) => Either::Left(self.cache.tags()[filter].ideas.iter()),
                    None => Either::Right(self.cache.ideas().keys()),
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
                    current_ideas: self.cache.read_relevant(idea)?,
                })?;
            }

            Message::OpenIdeaPicker { link } => {
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
                    kind: PickerKind::Idea { query, ideas, link },
                });
                return Ok(focus("picker_query"));
            }
            Message::OpenTagPicker => {
                let mut tags: Vec<_> = self.cache.tags().keys().cloned().collect();
                tags.sort_by_key(|tag| self.cache.tags()[tag].ideas.last());
                tags.reverse();
                self.view_manager.picker = Some(Picker {
                    selected: None,
                    kind: PickerKind::Tag { tags },
                });
            }

            Message::NewIdea { content } => {
                self.view_manager.insert(View::Editor(content))?;
                return Ok(focus("editor"));
            }
            Message::PickerQuery(new_query) => {
                let Some(Picker {
                    selected,
                    kind: PickerKind::Idea { query, ideas, .. },
                }) = &mut self.view_manager.picker
                else {
                    panic!()
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
                    PickerKind::Tag { tags } => (tags.len(), 4),
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
                    selected,
                }) = &mut self.view_manager.picker
                {
                    *selected = Some(selected.map_or(0, |selected| selected + 1))
                        .min(tags.len().checked_sub(1));
                }
            }
            Message::PickLeft => {
                if let Some(Picker {
                    kind: PickerKind::Tag { .. },
                    selected,
                }) = &mut self.view_manager.picker
                {
                    *selected = selected.and_then(|selected| selected.checked_sub(1));
                }
            }

            Message::KeyPress(key, modifiers) => {
                return Ok(Task::done(match (key.as_ref(), modifiers) {
                    (Key::Character("n"), keyboard::Modifiers::CTRL) => Message::NewIdea {
                        content: Content::new(),
                    },
                    (Key::Character("f"), keyboard::Modifiers::CTRL) => {
                        Message::OpenIdeaPicker { link: false }
                    }
                    (Key::Character("l"), keyboard::Modifiers::CTRL)
                        if let View::Editor(_) = &*self.view_manager =>
                    {
                        Message::OpenIdeaPicker { link: true }
                    }
                    (Key::Character("r"), keyboard::Modifiers::CTRL) => Message::OpenRandom,
                    (Key::Character("s"), keyboard::Modifiers::CTRL)
                        if let View::Editor(_) = &*self.view_manager =>
                    {
                        Message::Save
                    }
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
                            kind: PickerKind::Idea { ideas, link, .. },
                        }) = &self.view_manager.picker =>
                    {
                        if *link {
                            Message::AddLink(ideas[*selected].0)
                        } else {
                            Message::OpenIdea(ideas[*selected].0)
                        }
                    }
                    (Key::Named(key::Named::Enter), keyboard::Modifiers::NONE)
                        if let Some(Picker {
                            selected: Some(selected),
                            kind: PickerKind::Tag { tags },
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
                    (Key::Character("w"), keyboard::Modifiers::CTRL) => Message::HistoryClose,
                    (Key::Character("b"), keyboard::Modifiers::CTRL)
                        if let View::Editor(content) = &*self.view_manager
                            && content.cursor().selection.is_some() =>
                    {
                        Message::Bold
                    }
                    (Key::Character("i"), keyboard::Modifiers::CTRL)
                        if let View::Editor(content) = &*self.view_manager
                            && content.cursor().selection.is_some() =>
                    {
                        Message::Italic
                    }
                    (Key::Character("t"), modifiers)
                        if let View::Editor(content) = &*self.view_manager
                            && content.cursor().selection.is_some()
                            && modifiers
                                == keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT =>
                    {
                        Message::Tag
                    }

                    _ => return Ok(Task::none()),
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

            Message::Italic => self.view_manager.modify(|view| {
                let View::Editor(content) = view else {
                    panic!()
                };

                let Cursor {
                    position,
                    selection,
                } = content.cursor();

                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: max(position.index, selection.unwrap().index),
                    },
                    selection: None,
                });

                content.perform(text_editor::Action::Edit(Edit::Insert('_')));

                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: min(position.index, selection.unwrap().index),
                    },
                    selection: None,
                });

                content.perform(text_editor::Action::Edit(Edit::Insert('_')));
                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: position.index + 2,
                    },
                    selection: None,
                });
            })?,
            Message::Bold => self.view_manager.modify(|view| {
                let View::Editor(content) = view else {
                    panic!()
                };

                let Cursor {
                    position,
                    selection,
                } = content.cursor();

                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: max(position.index, selection.unwrap().index),
                    },
                    selection: None,
                });

                (0..2).for_each(|_| content.perform(text_editor::Action::Edit(Edit::Insert('*'))));

                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: min(position.index, selection.unwrap().index),
                    },
                    selection: None,
                });

                (0..2).for_each(|_| content.perform(text_editor::Action::Edit(Edit::Insert('*'))));
                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        index: position.index + 2,
                    },
                    selection: None,
                });
            })?,
            Message::Tag => self.view_manager.modify(|view| {
                let View::Editor(content) = view else {
                    panic!();
                };

                let selection = content.selection().unwrap();
                let tag = format!(
                    "#{} ",
                    selection.split_whitespace().collect::<Vec<_>>().join("_")
                );
                content.perform(text_editor::Action::Edit(Edit::Paste(Arc::new(tag))));
            })?,
            Message::List => self.view_manager.modify(|view| {
                let View::Editor(content) = view else {
                    panic!()
                };
                content.perform(text_editor::Action::Edit(Edit::Enter));
                content.perform(text_editor::Action::Edit(Edit::Paste(Arc::new(
                    "- ".to_string(),
                ))));
            })?,
            Message::NumberedList => self.view_manager.modify(|view| {
                let View::Editor(content) = view else {
                    panic!()
                };
                content.perform(text_editor::Action::Edit(Edit::Enter));
                content.perform(text_editor::Action::Edit(Edit::Paste(Arc::new(
                    "1. ".to_string(),
                ))));
            })?,
            Message::AddLink(idea) => {
                self.view_manager.modify(|view| {
                    let View::Editor(content) = view else {
                        panic!()
                    };
                    content.perform(text_editor::Action::Edit(Edit::Paste(Arc::new(format!(
                        "[[{idea}]]"
                    )))));
                })?;
                return Ok(Task::done(Message::Refocus));
            }
        }

        Ok(Task::none())
    }
}
