use core::slice;
use std::{
    borrow::Cow,
    iter::{self, Cloned},
    ops::Range,
    rc::Rc,
    sync::LazyLock,
    vec
};

use iced::{
    Alignment, Font, Length,
    advanced::text::highlighter,
    border,
    font::{self, Family},
    keyboard::{
        self,
        Key::{self},
        key
    },
    widget::{
        self, button, center, column, container, grid, mouse_area, opaque, rich_text, row, rule,
        scrollable, space, span, stack,
        text::{Highlighter, Rich, Wrapping},
        text_editor::{Binding, Content, Motion},
        text_input, tooltip
    }
};
use lucide_icons::{Icon, iced::icon_x};

use crate::{
    Element,
    Message::{self},
    Pokisona,
    cache::IdeaRef,
    markdown::{self, Highlighted, Markdown, MarkdownSpan, Modifiers},
    theme::{ButtonClass, CATPPUCCIN, CatppuccinFrappe, ContainerClass, TextClass},
    view_manager::{PickerKind, View}
};

static TITLE: LazyLock<Markdown> = LazyLock::new(|| Markdown::new(include_str!("../README.md")));
const SPACING: f32 = 8.0;
const IDEA_SIZE: f32 = 500.0;
pub const BASE_FONT_SIZE: f32 = 18.0;

impl Pokisona {
    pub fn view(&self) -> Element<'_> {
        let content: Element = match &*self.view_manager {
            View::Title => container(
                container(self.view_markdown(&TITLE))
                    .width(IDEA_SIZE)
                    .padding(SPACING)
                    .class(ContainerClass::BorderedBox { highlighted: false })
            )
            .padding(SPACING * 2.0)
            .into(),
            // TODO: add highlighting
            View::Editor {
                content,
                highlighted,
                ..
            } => widget::text_editor(content)
                .on_action(Message::Editor)
                .font(Font {
                    family: Family::Name("Libron"),
                    ..Default::default()
                })
                .key_binding(|press| match press.key {
                    Key::Named(key::Named::Escape) => None,
                    // TODO: Delete this after update to iced 0.15
                    Key::Named(key::Named::Backspace)
                        if press.modifiers == keyboard::Modifiers::CTRL =>
                    {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::WordLeft),
                            Binding::Backspace,
                        ]))
                    }
                    Key::Named(key::Named::Delete)
                        if press.modifiers == keyboard::Modifiers::CTRL =>
                    {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::WordRight),
                            Binding::Delete,
                        ]))
                    }
                    _ => Binding::from_key_press(press)
                })
                .wrapping(Wrapping::WordOrGlyph)
                .placeholder("Your idea...")
                .width(IDEA_SIZE)
                .height(IDEA_SIZE / 2.0)
                .padding(SPACING)
                .id("editor")
                .highlight_with::<MarkdownHighlighter>(highlighted.clone(), |highlight, _| {
                    *highlight
                })
                .into(),
            View::Idea {
                idea,
                current_ideas
            } => column![
                row(self.cache.ideas()[idea].links.iter().map(|idea| {
                    self.view_idea(
                        Some(*idea),
                        &current_ideas[idea],
                        ViewIdeaOptions::default()
                    )
                }))
                .spacing(SPACING)
                .align_y(Alignment::End)
                .height(Length::Fill),
                self.view_idea(
                    Some(*idea),
                    &current_ideas[idea],
                    ViewIdeaOptions {
                        enlarged: true,
                        highlighted: true
                    }
                ),
                row(self.cache.ideas()[idea]
                    .backlinks
                    .iter()
                    .map(|idea| self.view_idea(
                        Some(*idea),
                        &current_ideas[idea],
                        ViewIdeaOptions::default()
                    )))
                .spacing(SPACING)
                .height(Length::Fill),
            ]
            .spacing(SPACING)
            .align_x(Alignment::Center)
            .into()
        };

        #[allow(clippy::items_after_statements)]
        fn top_button(icon: Icon, message: Option<Message>, tooltip: &str) -> Element<'_> {
            let content: widget::Button<'_, Message, CatppuccinFrappe> =
                button(widget::Text::from(icon)).on_press_maybe(message);
            widget::tooltip(
                content,
                container(tooltip)
                    .class(ContainerClass::Surface1)
                    .padding(SPACING),
                tooltip::Position::Bottom
            )
            .gap(SPACING)
            .into()
        }

        let top_left_buttons = row![
            top_button(
                Icon::CircleX,
                self.view_manager
                    .can_close_current()
                    .then_some(Message::HistoryClose),
                "Close (Ctrl-w)"
            ),
            rule::vertical(1.0),
            top_button(
                Icon::ArrowLeft,
                self.view_manager
                    .can_go_backward()
                    .then_some(Message::HistoryBackward),
                "Go back (Alt-left)"
            ),
            top_button(
                Icon::ArrowRight,
                self.view_manager
                    .can_go_forward()
                    .then_some(Message::HistoryForward),
                "Go forward (Alt-right)"
            ),
        ]
        .spacing(SPACING);

        let top_right_buttons = if let View::Editor { .. } = &*self.view_manager {
            row![
                top_button(Icon::Save, Some(Message::Save), "Save (Ctrl-s)"),
                rule::vertical(1.0)
            ]
        } else {
            row![]
        }
        .extend([
            top_button(
                Icon::FilePlus,
                Some(Message::NewIdea {
                    content: Content::new()
                }),
                "New idea (Ctrl-n)"
            ),
            top_button(
                Icon::FileSearch,
                Some(Message::OpenIdeaPicker),
                "Open idea picker (Ctrl-f)"
            ),
            top_button(
                Icon::Tags,
                Some(Message::OpenTagPicker),
                "Open tag picker (Ctrl-t)"
            ),
            top_button(
                Icon::Dice3,
                Some(Message::OpenRandom),
                "Revisit a random idea (Ctrl-r)"
            )
        ])
        .height(Length::Shrink)
        .spacing(SPACING);
        let top_right_buttons = container(top_right_buttons)
            .align_right(Length::Fill)
            .padding(SPACING);
        let error = self
            .error
            .as_ref()
            .map(|error| widget::text(error).class(TextClass::Danger));
        let top_left = row![top_left_buttons, error]
            .spacing(SPACING)
            .padding(SPACING)
            .align_y(Alignment::Center);
        let picker = self.view_manager.picker.as_ref().map(|picker| {
            let content: Element<'_> = match &picker.kind {
                PickerKind::Idea { query, ideas } => column![
                    text_input("Enter query", query)
                        .id("picker_query")
                        .on_input(Message::PickerQuery)
                ]
                .extend(ideas.iter().enumerate().map(|(i, (idea, markdown))| {
                    let options = ViewIdeaOptions {
                        enlarged: false,
                        highlighted: picker.selected == Some(i)
                    };
                    self.view_idea(Some(*idea), markdown, options)
                }))
                .spacing(SPACING)
                .align_x(Alignment::Center)
                .into(),
                PickerKind::Tag { tags } => grid(tags.iter().enumerate().map(|(i, tag)| {
                    button(
                        container(widget::text!(
                            "#{tag} ({} ideas)",
                            self.cache.tags()[tag].ideas.len()
                        ))
                        .center_x(Length::Fill)
                    )
                    .class(ButtonClass::Tag {
                        highlight: picker.selected == Some(i),
                        color: self.cache.tags()[tag].color
                    })
                    .on_press(Message::SetTagFilter(tag.clone()))
                    .into()
                }))
                .height(Length::Shrink)
                .columns(4)
                .spacing(SPACING)
                .into()
            };

            container(opaque(
                container(content)
                    .padding(SPACING)
                    .center_x(IDEA_SIZE * 2.0)
                    .class(ContainerClass::Surface0)
            ))
            .center_x(Length::Fill)
            .padding(SPACING)
        });

        let overlay = self.view_manager.picker.is_some().then(|| {
            opaque(
                mouse_area(
                    container(space())
                        .class(ContainerClass::Tint)
                        .center(Length::Fill)
                )
                .on_press(Message::ClosePicker)
            )
        });
        stack![
            center(content).class(ContainerClass::Base),
            stack![
                top_right_buttons,
                container(self.tag_filter.as_ref().map(|tag| {
                    row![
                        "Filtering by",
                        container(&**tag).padding(button::DEFAULT_PADDING).class(
                            ContainerClass::Tag {
                                color: self.cache.tags()[tag].color
                            }
                        ),
                        button(icon_x())
                            .class(ButtonClass::Secondary)
                            .on_press(Message::UnsetTagFilter)
                    ]
                    .align_y(Alignment::Center)
                    .spacing(SPACING)
                }))
                .padding(SPACING)
                .center_x(Length::Fill),
                top_left,
            ],
            overlay,
            picker
        ]
        .into()
    }

    fn view_idea<'a>(
        &'a self,
        idea: Option<IdeaRef>,
        markdown: &'a Markdown,
        options: ViewIdeaOptions
    ) -> Element<'a> {
        let size = BASE_FONT_SIZE * if options.enlarged { 1.5 } else { 1.0 };
        let text_span = |text: Cow<'a, str>, modifiers: Modifiers| {
            widget::span(text).font(Font {
                weight: if modifiers.contains(Modifiers::BOLD) {
                    font::Weight::Bold
                } else {
                    font::Weight::Normal
                },
                style: if modifiers.contains(Modifiers::ITALIC) {
                    font::Style::Italic
                } else {
                    font::Style::Normal
                },
                family: Family::Name("Libron"),
                ..Default::default()
            })
        };
        let spans = markdown
            .lines()
            .iter()
            .flat_map(|line| {
                iter::once(span('\n'))
                    .chain(line.list_item.map(|(_, item)| {
                        match item {
                            markdown::ListItem::Bullet => {
                                text_span(" • ".into(), Modifiers::ITALIC)
                            }
                            markdown::ListItem::Number(number) => {
                                text_span(format!(" {number}.").into(), Modifiers::ITALIC)
                            }
                        }
                        .color(CATPPUCCIN.blue)
                    }))
                    .chain(line.spans.iter().map(|(_, span)| {
                        match span {
                            MarkdownSpan::Text(span, modifiers) => {
                                let color =
                                    (*modifiers != Modifiers::NONE).then_some(CATPPUCCIN.blue);
                                text_span(span.into(), *modifiers).color_maybe(color)
                            }

                            MarkdownSpan::Link {
                                display,
                                target,
                                modifiers
                            } => text_span(
                                display
                                    .as_deref()
                                    .map(Cow::from)
                                    .unwrap_or(target.to_string().into()),
                                *modifiers
                            )
                            .color(CATPPUCCIN.blue)
                            .link(Link::Idea(*target)),
                            MarkdownSpan::ModifierDelimeter => "".into(),
                            MarkdownSpan::Tag(tag) => widget::span(format!("#{tag}"))
                                .background(CATPPUCCIN[self.cache.tags()[tag].color])
                                .link(Link::Tag(tag.clone()))
                                .color(CATPPUCCIN.crust)
                                .border(border::rounded(2))
                        }
                    }))
            })
            .skip(1);

        let markdown = { Rich::from_iter(spans) }
            .on_link_click(|link| match link {
                Link::Idea(idea) => Message::OpenIdea(idea),
                Link::Tag(tag) => Message::SetTagFilter(tag)
            })
            .size(size);
        let scale = if options.enlarged { 1.5 } else { 1.0 };
        container(scrollable(
            column![
                idea.map(|idea| rich_text![
                    span(idea.to_string())
                        .color(CATPPUCCIN.blue)
                        .size(BASE_FONT_SIZE * scale * 1.2)
                        .link(idea)
                ]
                .on_link_click(Message::OpenIdea)),
                idea.is_some().then_some(rule::horizontal(1)),
                markdown
            ]
            .spacing(SPACING)
        ))
        .class(ContainerClass::BorderedBox {
            highlighted: options.highlighted
        })
        .width(IDEA_SIZE * scale)
        .padding(SPACING)
        .into()
    }

    fn view_markdown<'a>(
        &self,
        markdown: &'a Markdown
    ) -> Rich<'a, Link, Message, CatppuccinFrappe> {
        let text_span = |text: Cow<'a, str>, modifiers: Modifiers| {
            widget::span(text).font(Font {
                weight: if modifiers.contains(Modifiers::BOLD) {
                    font::Weight::Bold
                } else {
                    font::Weight::Normal
                },
                style: if modifiers.contains(Modifiers::ITALIC) {
                    font::Style::Italic
                } else {
                    font::Style::Normal
                },
                family: Family::Name("Libron"),
                ..Default::default()
            })
        };

        let spans = markdown
            .lines()
            .iter()
            .flat_map(|line| {
                iter::once(span('\n'))
                    .chain(line.list_item.map(|(_, item)| {
                        match item {
                            markdown::ListItem::Bullet => {
                                text_span(" • ".into(), Modifiers::ITALIC)
                            }
                            markdown::ListItem::Number(number) => {
                                text_span(format!(" {number}.").into(), Modifiers::ITALIC)
                            }
                        }
                        .color(CATPPUCCIN.blue)
                    }))
                    .chain(line.spans.iter().map(|(_, span)| {
                        match span {
                            MarkdownSpan::Text(span, modifiers) => {
                                let color =
                                    (*modifiers != Modifiers::NONE).then_some(CATPPUCCIN.blue);
                                text_span(span.into(), *modifiers).color_maybe(color)
                            }

                            MarkdownSpan::Link {
                                display,
                                target,
                                modifiers
                            } => text_span(
                                display
                                    .as_deref()
                                    .map(Cow::from)
                                    .unwrap_or(target.to_string().into()),
                                *modifiers
                            )
                            .color(CATPPUCCIN.blue)
                            .link(Link::Idea(*target)),
                            MarkdownSpan::ModifierDelimeter => "".into(),
                            MarkdownSpan::Tag(tag) => widget::span(format!("#{tag}"))
                                .background(CATPPUCCIN[self.cache.tags()[tag].color])
                                .link(Link::Tag(tag.clone()))
                                .color(CATPPUCCIN.crust)
                                .border(border::rounded(2))
                        }
                    }))
            })
            .skip(1);
        Rich::from_iter(spans)
    }
}

#[derive(Clone)]
enum Link {
    Idea(IdeaRef),
    Tag(String)
}

#[derive(Default, Clone, Copy)]
struct ViewIdeaOptions {
    enlarged: bool,
    highlighted: bool
}

struct MarkdownHighlighter {
    current_line: usize,
    highlighted: Rc<Highlighted>
}

impl Highlighter for MarkdownHighlighter {
    type Settings = Rc<Highlighted>;

    type Highlight = highlighter::Format<Font>;

    type Iterator<'a>
        = Cloned<slice::Iter<'a, (Range<usize>, Self::Highlight)>>
    where
        Self: 'a;

    fn new(highlighted: &Self::Settings) -> Self {
        Self {
            current_line: 0,
            highlighted: highlighted.clone()
        }
    }

    fn update(&mut self, highlighted: &Self::Settings) {
        self.highlighted = highlighted.clone();
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, _line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        self.highlighted.0[self.current_line - 1].iter().cloned()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}
