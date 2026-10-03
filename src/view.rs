use std::{borrow::Cow, iter};

use iced::{
    Alignment, Font, Length, border,
    font::{self, Family},
    widget::{
        self, button, center, column, container, grid, mouse_area, opaque, rich_text, row, rule,
        scrollable, space, span, stack,
        text::{Rich, Wrapping},
        text_editor::Content,
        text_input, tooltip
    }
};
use lucide_icons::{Icon, iced::icon_x};

use crate::{
    Element,
    Message::{self},
    PickerKind, Pokisona, View,
    cache::IdeaRef,
    markdown::{self, Markdown, MarkdownSpan, Modifiers},
    theme::{ButtonClass, CATPPUCCIN, CatppuccinFrappe, ContainerClass, TextClass}
};

impl Pokisona {
    const SPACING: f32 = 8.0;
    const IDEA_SIZE: f32 = 500.0;
    pub const BASE_FONT_SIZE: f32 = 18.0;
    pub fn view(&self) -> Element<'_> {
        let content: Element = match self.history.current_view() {
            View::Title(markdown) => container(
                container(self.view_markdown(markdown))
                    .width(Self::IDEA_SIZE)
                    .padding(Self::SPACING)
                    .class(ContainerClass::BorderedBox { highlighted: false })
            )
            .padding(Self::SPACING * 2.0)
            .into(),
            // TODO: add highlighting
            View::NewIdea { content, .. } => widget::text_editor(content)
                .on_action(Message::Editor)
                .font(Font {
                    family: Family::Name("Libron"),
                    ..Default::default()
                })
                .wrapping(Wrapping::WordOrGlyph)
                .placeholder("Your idea...")
                .width(Self::IDEA_SIZE)
                .height(Self::IDEA_SIZE / 2.0)
                .padding(Self::SPACING)
                .id("editor")
                .into(),
            View::Idea {
                idea,
                markdown: parsed,
                links,
                backlinks
            } => column![
                row(links.iter().map(|(idea, markdown)| {
                    self.view_idea(Some(*idea), markdown, ViewIdeaOptions::default())
                }))
                .spacing(Self::SPACING)
                .align_y(Alignment::End)
                .height(Length::Fill),
                self.view_idea(
                    Some(*idea),
                    parsed,
                    ViewIdeaOptions {
                        enlarged: true,
                        highlighted: true
                    }
                ),
                row(backlinks.iter().map(|(idea, markdown)| self.view_idea(
                    Some(*idea),
                    markdown,
                    ViewIdeaOptions::default()
                )))
                .spacing(Self::SPACING)
                .height(Length::Fill),
            ]
            .spacing(Self::SPACING)
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
                    .padding(Pokisona::SPACING),
                tooltip::Position::Bottom
            )
            .gap(Pokisona::SPACING)
            .into()
        }

        let buttons = match self.history.current_view() {
            View::NewIdea { .. } => {
                row![
                    top_button(Icon::Save, Some(Message::Save), "Save (Ctrl-s)"),
                    rule::vertical(1.0)
                ]
            }
            View::Idea { idea, .. } => row![
                top_button(
                    Icon::Link,
                    Some(Message::CopyLink(*idea)),
                    "Copy link (Ctrl-c)"
                ),
                top_button(Icon::Reply, Some(Message::Reply(*idea)), "Reply (Ctrl-p)"),
                rule::vertical(1.0)
            ],
            View::Title(_) => row![]
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
            ),
            rule::vertical(1.0).into(),
            top_button(
                Icon::ArrowLeft,
                self.history
                    .can_go_backward()
                    .then_some(Message::HistoryBackward),
                "Go back (Alt-left)"
            ),
            top_button(
                Icon::ArrowRight,
                self.history
                    .can_go_forward()
                    .then_some(Message::HistoryForward),
                "Go forward (Alt-right)"
            ),
            top_button(
                Icon::CircleX,
                self.history
                    .can_close_current()
                    .then_some(Message::HistoryClose),
                "Close (Ctrl-w)"
            )
        ])
        .height(Length::Shrink)
        .spacing(Self::SPACING);
        let buttons = container(buttons)
            .align_right(Length::Fill)
            .padding(Self::SPACING);
        let error = self
            .error
            .as_ref()
            .map(|error| widget::text(error).class(TextClass::Danger));

        let picker = self.picker.as_ref().map(|picker| {
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
                .spacing(Self::SPACING)
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
                .spacing(Self::SPACING)
                .into()
            };

            container(opaque(
                container(content)
                    .padding(Self::SPACING)
                    .center_x(Self::IDEA_SIZE * 2.0)
                    .class(ContainerClass::Surface0)
            ))
            .center_x(Length::Fill)
            .padding(Self::SPACING)
        });

        let overlay = self.picker.is_some().then(|| {
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
                container(buttons).align_right(Length::Fill),
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
                    .spacing(Self::SPACING)
                }))
                .padding(Self::SPACING)
                .center_x(Length::Fill),
                error,
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
        let size = Self::BASE_FONT_SIZE * if options.enlarged { 1.5 } else { 1.0 };
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
                    .chain(line.list_item.map(|item| {
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
                    .chain(line.spans.iter().map(|span| {
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
                            MarkdownSpan::ModifierDelimeter(_) => "".into(),
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
                        .size(Self::BASE_FONT_SIZE * scale * 1.2)
                        .link(idea)
                ]
                .on_link_click(Message::OpenIdea)),
                idea.is_some().then_some(rule::horizontal(1)),
                markdown
            ]
            .spacing(Self::SPACING)
        ))
        .class(ContainerClass::BorderedBox {
            highlighted: options.highlighted
        })
        .width(Self::IDEA_SIZE * scale)
        .padding(Self::SPACING)
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
                    .chain(line.list_item.map(|item| {
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
                    .chain(line.spans.iter().map(|span| {
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
                            MarkdownSpan::ModifierDelimeter(_) => "".into(),
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
