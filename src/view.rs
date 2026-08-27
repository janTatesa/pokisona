use std::{borrow::Cow, iter};

use iced::{
    Alignment, Color, Font, Length, Theme, border,
    font::{self, Family},
    widget::{
        self, button, center, column, container, mouse_area, opaque, rich_text, row, rule,
        scrollable, space, span, stack,
        text::{self, Rich, Wrapping},
        text_editor::{self},
        text_input
    }
};
use lucide_icons::iced::{icon_dice_3, icon_file_plus, icon_file_search, icon_link};

use crate::{
    Element,
    Message::{self},
    Pokisona, View,
    cache::IdeaRef,
    markdown::{self, Markdown, MarkdownSpan, Modifiers}
};

impl Pokisona {
    const SPACING: f32 = 8.0;
    const IDEA_SIZE: f32 = 500.0;
    pub const BASE_FONT_SIZE: f32 = 18.0;
    pub fn view(&self) -> Element<'_> {
        let content: Element = match &self.view {
            View::Title(markdown) => self.view_markdown(markdown).size(32.0).into(),
            // TODO: add highlighting
            View::NewIdea { content, .. } => widget::text_editor(content)
                .on_action(Message::Editor)
                .font(Font {
                    family: Family::Serif,
                    ..Default::default()
                })
                .style(|theme: &Theme, _| text_editor::Style {
                    background: theme.extended_palette().background.weakest.color.into(),
                    border: border::rounded(5.0),
                    placeholder: theme.extended_palette().secondary.base.color,
                    value: theme.palette().text,
                    selection: theme.extended_palette().primary.base.color.scale_alpha(0.2)
                })
                .wrapping(Wrapping::WordOrGlyph)
                .placeholder("Your idea...")
                .width(Self::IDEA_SIZE)
                .height(Self::IDEA_SIZE)
                .padding(Self::SPACING)
                .id("editor")
                .into(),
            View::Revisiting {
                idea,
                markdown: parsed,
                links,
                backlinks
            } => column![
                row(links.iter().map(|(idea, markdown)| self.view_idea(
                    *idea,
                    markdown,
                    ViewIdeaOptions::default()
                )))
                .align_y(Alignment::End),
                self.view_idea(
                    *idea,
                    parsed,
                    ViewIdeaOptions {
                        enlarged: true,
                        highlighted: true
                    }
                ),
                row(backlinks.iter().map(|(idea, markdown)| self.view_idea(
                    *idea,
                    markdown,
                    ViewIdeaOptions::default()
                ))),
            ]
            .spacing(Self::SPACING)
            .into()
        };

        let buttons = row![
            button(icon_file_plus()).on_press(Message::NewIdea),
            rule::vertical(1.0),
            button(icon_link()).on_press_maybe(if let View::Revisiting { idea, .. } = self.view {
                Some(Message::CopyLink(idea))
            } else {
                None
            }),
            rule::vertical(1.0),
            button(icon_file_search()).on_press(Message::OpenPicker),
            button(icon_dice_3()).on_press(Message::RevisitRandom)
        ]
        .height(Length::Shrink)
        .spacing(Self::SPACING);
        let buttons = container(buttons)
            .align_right(Length::Fill)
            .padding(Self::SPACING);
        let error = self
            .error
            .as_ref()
            .map(|error| widget::text(error).style(text::danger));
        stack![
            center(content),
            row![error, buttons].align_y(Alignment::Center),
            self.picker.is_some().then(|| opaque(
                mouse_area(
                    container(space())
                        .style(|_| container::background(Color::BLACK.scale_alpha(0.8)))
                        .center(Length::Fill)
                )
                .on_press(Message::ClosePicker)
            )),
            self.picker.as_ref().map(|picker| opaque(
                container(
                    container(
                        column![
                            text_input("Enter query", &picker.query)
                                .id("picker_query")
                                .on_input(Message::PickerQuery)
                                .on_submit_maybe(
                                    picker.selected.map(|selected| Message::Open(
                                        picker.top_entries[selected].0
                                    ))
                                )
                        ]
                        .extend(picker.top_entries.iter().enumerate().map(
                            |(i, (idea, markdown))| self.view_idea(
                                *idea,
                                markdown,
                                ViewIdeaOptions {
                                    enlarged: false,
                                    highlighted: picker.selected == Some(i)
                                }
                            )
                        ))
                        .align_x(Alignment::Center)
                        .spacing(Self::SPACING)
                    )
                    .padding(Self::SPACING)
                    .center_x(Self::IDEA_SIZE * 2.0)
                    .style(container::bordered_box)
                )
                .center_x(Length::Fill)
                .padding(Self::SPACING)
            ))
        ]
        .into()
    }

    fn view_idea<'a>(
        &'a self,
        idea: IdeaRef,
        markdown: &'a Markdown,
        options: ViewIdeaOptions
    ) -> Element<'a> {
        let size = if options.enlarged {
            Self::BASE_FONT_SIZE * 1.5
        } else {
            Self::BASE_FONT_SIZE
        };
        let markdown = self
            .view_markdown(markdown)
            .on_link_click(Message::Open)
            .size(size);
        container(scrollable(
            column![
                rich_text![span(idea.to_string()).link(idea)].on_link_click(Message::Open),
                rule::horizontal(1),
                markdown
            ]
            .spacing(Self::SPACING)
        ))
        .style(move |theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weakest.color.into()),
            border: border::color(if options.highlighted {
                theme.palette().primary
            } else {
                theme.extended_palette().secondary.base.color
            })
            .rounded(5)
            .width(2),
            ..Default::default()
        })
        .width(Self::IDEA_SIZE)
        .max_height(Self::IDEA_SIZE)
        .padding(Self::SPACING)
        .into()
    }

    fn view_markdown<'a>(&self, markdown: &'a Markdown) -> Rich<'a, IdeaRef, Message> {
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
                family: Family::Serif,
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
                        .color(self.theme().palette().primary)
                    }))
                    .chain(line.spans.iter().map(|span| {
                        match span {
                            MarkdownSpan::Text(span, modifiers) => {
                                let color = (*modifiers != Modifiers::NONE)
                                    .then_some(self.theme().palette().primary);
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
                            .color(self.theme().palette().primary)
                            .link(*target),
                            MarkdownSpan::ModifierDelimeter(_) => todo!()
                        }
                    }))
            })
            .skip(1);
        Rich::from_iter(spans)
    }
}

#[derive(Default, Clone, Copy)]
struct ViewIdeaOptions {
    enlarged: bool,
    highlighted: bool
}
