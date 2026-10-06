use std::{
    borrow::Cow,
    iter::{self},
    ops::Range,
    sync::LazyLock,
    vec,
};

use chumsky::Parser;
use iced::{
    Alignment, Element, Font, Length, Widget,
    advanced::text::highlighter,
    border,
    font::{self},
    keyboard::{
        Key::{self},
        key,
    },
    widget::{
        self, button, center, column, container, grid, mouse_area, opaque, rich_text, row, rule,
        scrollable, space, span, stack,
        text::{self, Highlighter, Rich, Wrapping},
        text_editor::{Binding, Content},
        text_input, tooltip,
    },
};
use lucide_icons::Icon;

use crate::{
    Message::{self},
    Pokisona,
    cache::{Cache, IdeaRef},
    markdown::{self, Markdown, MarkdownLine, MarkdownSpan, Modifiers},
    theme::{ButtonClass, CATPPUCCIN, CatppuccinFrappe, ContainerClass, TextClass},
    view_manager::{PickerKind, View},
};

static TITLE: LazyLock<Markdown> = LazyLock::new(|| Markdown::new(include_str!("../README.md")));
const SPACING: f32 = 8.0;
const IDEA_SIZE: f32 = 600.0;
pub const BASE_FONT_SIZE: f32 = 18.0;

impl Pokisona {
    pub fn view(&self) -> impl Widget<Message, CatppuccinFrappe> {
        let content: Element<_, _> = match &*self.view_manager {
            View::Title => container(self.view_idea(
                None,
                &TITLE,
                ViewIdeaOptions {
                    enlarged: true,
                    highlighted: false,
                },
                Message::OpenIdea,
            ))
            .center(Length::Fill)
            .boxed(),

            View::Editor(content) => {
                let on_selection = |m: Message| content.cursor().selection.is_some().then_some(m);
                column![
                    (!self.zen_mode).then(|| row![
                        button_helper(
                            Icon::Italic,
                            on_selection(Message::Italic),
                            "Italic (Ctrl-i)",
                            ButtonClass::Secondary
                        ),
                        button_helper(
                            Icon::Bold,
                            on_selection(Message::Bold),
                            "Bold (Ctrl-b)",
                            ButtonClass::Secondary
                        ),
                        button_helper(
                            Icon::Tag,
                            on_selection(Message::Tag),
                            "Tag (Ctrl-shift-t)",
                            ButtonClass::Secondary
                        ),
                        space().width(Length::Fill),
                        button_helper(
                            Icon::Link,
                            Some(Message::OpenIdeaPicker { link: true }),
                            "Link (Ctrl-l)",
                            ButtonClass::Secondary
                        ),
                        button_helper(
                            Icon::List,
                            Some(Message::List),
                            "List",
                            ButtonClass::Secondary
                        ),
                        button_helper(
                            Icon::ListOrdered,
                            Some(Message::NumberedList),
                            "Numbered list",
                            ButtonClass::Secondary
                        )
                    ]
                    .spacing(SPACING)
                    .height(Length::Shrink)),
                    widget::text_editor(content)
                        .on_action(Message::Editor)
                        .key_binding(|press| match press.key {
                            Key::Named(key::Named::Escape) => None,

                            _ => Binding::from_key_press(press),
                        })
                        .wrapping(Wrapping::WordOrGlyph)
                        .placeholder("Your idea...")
                        .font(Font {
                            family: font::Family::Name("Libron"),
                            ..Default::default()
                        })
                        .width(IDEA_SIZE)
                        .height(IDEA_SIZE / 2.0)
                        .padding(SPACING)
                        .id("editor")
                        .highlight_with::<MarkdownParser>((), MarkdownHighlighter(&self.cache))
                ]
                .spacing(SPACING)
                .width(Length::Shrink)
                .boxed()
            }
            View::Idea {
                idea,
                current_ideas,
            } => column![
                row(self.cache.ideas()[idea].links.iter().map(|idea| {
                    self.view_idea(
                        Some(*idea),
                        &current_ideas[idea],
                        ViewIdeaOptions::default(),
                        Message::OpenIdea,
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
                    },
                    Message::OpenIdea
                ),
                row(self.cache.ideas()[idea]
                    .backlinks
                    .iter()
                    .map(|idea| self.view_idea(
                        Some(*idea),
                        &current_ideas[idea],
                        ViewIdeaOptions::default(),
                        Message::OpenIdea
                    ),))
                .spacing(SPACING)
                .height(Length::Fill),
            ]
            .spacing(SPACING)
            .align_x(Alignment::Center)
            .boxed(),
        };

        #[allow(clippy::items_after_statements)]
        fn button_helper(
            icon: Icon,
            message: Option<Message>,
            tooltip: &str,
            class: ButtonClass,
        ) -> impl Widget<Message, CatppuccinFrappe> {
            let content = button(widget::text(icon.unicode()).font(Font::with_family("lucide")))
                .on_press_maybe(message)
                .class(class);
            widget::tooltip(
                content,
                container(tooltip)
                    .class(ContainerClass::Surface1)
                    .padding(button::DEFAULT_PADDING),
                tooltip::Position::Bottom,
            )
            .gap(SPACING)
        }

        let page_indicator = container(
            row![
                widget::text(self.view_manager.index() + 1),
                widget::text("/").class(TextClass::Overlay0),
                widget::text(self.view_manager.total_views())
            ]
            .spacing(SPACING),
        )
        .padding(button::DEFAULT_PADDING)
        .class(ContainerClass::Surface0);
        let top_left_buttons = row![
            button_helper(
                Icon::CircleX,
                self.view_manager
                    .can_close_current()
                    .then_some(Message::HistoryClose),
                "Close (Ctrl-w)",
                ButtonClass::Primary
            ),
            rule::vertical(1.0),
            button_helper(
                Icon::ArrowLeft,
                self.view_manager
                    .can_go_backward()
                    .then_some(Message::HistoryBackward),
                "Go back (Alt-left)",
                ButtonClass::Primary
            ),
            button_helper(
                Icon::ArrowRight,
                self.view_manager
                    .can_go_forward()
                    .then_some(Message::HistoryForward),
                "Go forward (Alt-right)",
                ButtonClass::Primary
            ),
            rule::vertical(1.0)
        ]
        .spacing(SPACING);

        let error = self
            .error
            .as_ref()
            .map(|error| widget::text(error).class(TextClass::Danger));

        let top_left = row![
            (!self.zen_mode).then_some(top_left_buttons),
            page_indicator,
            error
        ]
        .spacing(SPACING)
        .padding(SPACING)
        .align_y(Alignment::Center);

        let top_right_buttons = if let View::Editor { .. } = &*self.view_manager {
            row![
                button_helper(
                    Icon::Save,
                    Some(Message::Save),
                    "Save (Ctrl-s)",
                    ButtonClass::Primary
                ),
                rule::vertical(1.0)
            ]
        } else {
            row![]
        }
        .extend([
            button_helper(
                Icon::FilePlus,
                Some(Message::NewIdea {
                    content: Content::new(),
                }),
                "New idea (Ctrl-n)",
                ButtonClass::Primary,
            )
            .boxed(),
            button_helper(
                Icon::FileSearch,
                Some(Message::OpenIdeaPicker { link: false }),
                "Open idea picker (Ctrl-f)",
                ButtonClass::Primary,
            )
            .boxed(),
            button_helper(
                Icon::Tags,
                Some(Message::OpenTagPicker),
                "Open tag picker (Ctrl-t)",
                ButtonClass::Primary,
            )
            .boxed(),
            button_helper(
                Icon::Dice3,
                Some(Message::OpenRandom),
                "Revisit a random idea (Ctrl-r)",
                ButtonClass::Primary,
            )
            .boxed(),
            button_helper(
                Icon::Keyboard,
                Some(Message::ToggleZenMode),
                "Toggle zen mode (Ctrl-z)",
                ButtonClass::Primary,
            )
            .boxed(),
        ])
        .height(Length::Shrink)
        .spacing(SPACING);
        let top_right_buttons = container(top_right_buttons)
            .align_right(Length::Fill)
            .padding(SPACING);
        let picker = self.view_manager.picker.as_ref().map(|picker| {
            let content = match &picker.kind {
                PickerKind::Idea { query, ideas, link } => column![
                    text_input("Enter query", query)
                        .id("picker_query")
                        .on_input(Message::PickerQuery)
                ]
                .extend(ideas.iter().enumerate().map(|(i, (idea, markdown))| {
                    let options = ViewIdeaOptions {
                        enlarged: false,
                        highlighted: picker.selected == Some(i),
                    };
                    self.view_idea(
                        Some(*idea),
                        markdown,
                        options,
                        if *link {
                            Message::AddLink
                        } else {
                            Message::OpenIdea
                        },
                    )
                    .boxed()
                }))
                .spacing(SPACING)
                .align_x(Alignment::Center)
                .boxed(),
                PickerKind::Tag { tags } => grid(tags.iter().enumerate().map(|(i, tag)| {
                    button(
                        container(widget::text!(
                            "#{tag} ({} ideas)",
                            self.cache.tags()[tag].ideas.len()
                        ))
                        .center_x(Length::Fill),
                    )
                    .class(ButtonClass::Tag {
                        highlight: picker.selected == Some(i),
                        color: self.cache.tags()[tag].color,
                    })
                    .on_press(Message::SetTagFilter(tag.clone()))
                }))
                .height(Length::Shrink)
                .columns(4)
                .spacing(SPACING)
                .boxed(),
            };

            container(opaque(
                container(content)
                    .padding(SPACING)
                    .center_x(IDEA_SIZE * 2.0)
                    .class(ContainerClass::Surface0),
            ))
            .center_x(Length::Fill)
            .padding(SPACING)
        });

        let overlay = self.view_manager.picker.is_some().then(|| {
            opaque(
                mouse_area(
                    container(space())
                        .class(ContainerClass::Tint)
                        .center(Length::Fill),
                )
                .on_press(Message::ClosePicker),
            )
        });
        stack![
            center(content).class(ContainerClass::Base),
            stack![
                (!self.zen_mode).then_some(top_right_buttons),
                container(self.tag_filter.as_ref().map(|tag| {
                    row![
                        "Filtering by",
                        container(&**tag).padding(button::DEFAULT_PADDING).class(
                            ContainerClass::Tag {
                                color: self.cache.tags()[tag].color
                            }
                        ),
                        (!self.zen_mode).then_some(button_helper(
                            Icon::X,
                            Some(Message::UnsetTagFilter),
                            "Unset (esc)",
                            ButtonClass::Secondary
                        ))
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
    }

    fn view_idea<'a>(
        &'a self,
        idea: Option<IdeaRef>,
        markdown: &'a Markdown,
        options: ViewIdeaOptions,
        on_idea_link_click: fn(IdeaRef) -> Message,
    ) -> impl Widget<Message, CatppuccinFrappe> {
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
                family: font::Family::Name("Libron"),
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
                                modifiers,
                            } => text_span(
                                display
                                    .as_deref()
                                    .map(Cow::from)
                                    .unwrap_or(target.to_string().into()),
                                *modifiers,
                            )
                            .color(CATPPUCCIN.blue)
                            .link(Link::Idea(*target)),
                            MarkdownSpan::ModifierDelimeter => "".into(),
                            MarkdownSpan::Tag(tag) => widget::span(format!("#{tag}"))
                                .background(CATPPUCCIN[self.cache.tags()[tag].color])
                                .link(Link::Tag(tag.clone()))
                                .color(CATPPUCCIN.crust)
                                .border(border::rounded(2)),
                        }
                    }))
            })
            .skip(1);

        let markdown = { spans.collect::<Rich<_, _, CatppuccinFrappe>>() }
            .on_link_click(move |link| match link {
                Link::Idea(idea) => on_idea_link_click(idea),
                Link::Tag(tag) => Message::SetTagFilter(tag),
            })
            .selectable(true)
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
            .spacing(SPACING),
        ))
        .class(ContainerClass::BorderedBox {
            highlighted: options.highlighted,
        })
        .width(IDEA_SIZE * scale)
        .padding(SPACING)
    }
}

#[derive(Clone)]
enum Link {
    Idea(IdeaRef),
    Tag(String),
}

#[derive(Default, Clone, Copy)]
struct ViewIdeaOptions {
    enlarged: bool,
    highlighted: bool,
}

struct MarkdownParser {
    current_line: usize,
}

#[derive(Clone)]
enum ParserOutput {
    ListItemStart,
    Span(MarkdownSpan),
}

impl text::Parser for MarkdownParser {
    type Settings = ();

    type Output = ParserOutput;

    type Iterator<'a>
        = vec::IntoIter<(Range<usize>, ParserOutput)>
    where
        Self: 'a;

    fn new((): &Self::Settings) -> Self {
        Self { current_line: 0 }
    }

    fn update(&mut self, (): &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn parse_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let markdown_line = MarkdownLine::parser().parse(line).unwrap();
        let iter: Vec<_> = markdown_line
            .list_item
            .map(|(range, _)| (range.into_range(), ParserOutput::ListItemStart))
            .into_iter()
            .chain(
                markdown_line
                    .spans
                    .into_iter()
                    .map(|(range, span)| (range.into_range(), ParserOutput::Span(span))),
            )
            .collect();
        self.current_line += 1;
        iter.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

struct MarkdownHighlighter<'a>(&'a Cache);
impl Highlighter<ParserOutput, CatppuccinFrappe> for MarkdownHighlighter<'_> {
    fn id(&self) -> &'static str {
        "markdown highlighter"
    }

    fn highlight(&self, input: ParserOutput, _theme: &CatppuccinFrappe) -> highlighter::Style {
        match input {
            ParserOutput::ListItemStart => highlighter::Style {
                color: Some(CATPPUCCIN.blue.into()),
                style: None,
            },
            ParserOutput::Span(MarkdownSpan::ModifierDelimeter) => highlighter::Style {
                color: Some(CATPPUCCIN.overlay0.into()),
                style: None,
            },
            ParserOutput::Span(MarkdownSpan::Text(_, modifiers)) => highlighter::Style {
                color: (modifiers != Modifiers::NONE).then_some(CATPPUCCIN.blue.into()),
                style: Some(if modifiers.contains(Modifiers::ITALIC) {
                    font::Style::Italic
                } else {
                    font::Style::Normal
                }),
            },

            ParserOutput::Span(MarkdownSpan::Tag(tag)) => {
                let color = self
                    .0
                    .tags()
                    .get(&tag)
                    .map_or(self.0.next_tag_color(), |tag| tag.color);
                highlighter::Style {
                    color: Some(CATPPUCCIN[color].into()),
                    style: None,
                }
            }
            ParserOutput::Span(MarkdownSpan::Link { modifiers, .. }) => highlighter::Style {
                color: Some(CATPPUCCIN.blue.into()),
                style: Some(if modifiers.contains(Modifiers::ITALIC) {
                    font::Style::Italic
                } else {
                    font::Style::Normal
                }),
            },
        }
    }
}
