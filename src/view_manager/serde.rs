use std::{io, rc::Rc};

use iced::widget::text_editor::{Content, Cursor, Position};
use serde::{Deserialize, Serialize};

use crate::{
    cache::{Cache, IdeaRef},
    markdown::{Highlighted, Markdown},
    view_manager::View
};

#[derive(Deserialize, Serialize)]
#[allow(private_interfaces)]
pub enum ViewSerde {
    Title,
    NewIdea {
        text: String,
        position: PositionSerde,
        selection: Option<PositionSerde>
    },
    Idea(IdeaRef)
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct PositionSerde {
    line: usize,
    column: usize
}

impl ViewSerde {
    pub fn to_normal(&self, cache: &mut Cache) -> io::Result<View> {
        Ok(match self {
            Self::Title => View::Title,
            Self::NewIdea {
                text,
                position,
                selection
            } => {
                let mut content = Content::with_text(text);
                content.move_to(Cursor {
                    position: Position {
                        line: position.line,
                        column: position.column
                    },
                    selection: selection.map(|selection| Position {
                        line: selection.line,
                        column: selection.column
                    })
                });
                let highlighted = Rc::new(Highlighted::new(&Markdown::new(text), cache));
                View::Editor {
                    content,
                    highlighted
                }
            }
            Self::Idea(idea) => View::Idea {
                idea: *idea,
                current_ideas: cache.read_relevant(*idea)?
            }
        })
    }
}

impl View {
    pub fn to_serde(&self) -> ViewSerde {
        match self {
            View::Title => ViewSerde::Title,
            View::Editor { content, .. } => {
                let Cursor {
                    position,
                    selection
                } = content.cursor();
                ViewSerde::NewIdea {
                    text: content.text(),
                    position: PositionSerde {
                        line: position.line,
                        column: position.column
                    },
                    selection: selection.map(|selection| PositionSerde {
                        line: selection.line,
                        column: selection.column
                    })
                }
            }
            View::Idea { idea, .. } => ViewSerde::Idea(*idea)
        }
    }
}
