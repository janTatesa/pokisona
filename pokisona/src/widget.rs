//! Widgets for pokisona
//! The reason this is necessary is to enforce a consistent UI, the performance cost is worth it.
//! An alternative would be to create fiunctions but that wouldn't be enforced
//! Just like the rest of the project this is shitty and incomplete
mod theme;
use std::{borrow::Cow, iter};

use iced::{
    Alignment, Background, Border, Color, Element, Font,
    Length::{Fill, Shrink},
    Padding, Pixels, Shadow, Vector,
    advanced::widget::text,
    font::{self, Weight},
    never,
    widget::{self, Column, Row, container, rule::FillMode, text_input}
};
use pokisona_markdown::Modifiers;
pub use theme::Theme;

use crate::app::{Message, TextInputId};

#[derive(Debug)]
pub enum Widget<'a> {
    TextInput {
        content: &'a str,
        placeholder: &'a str,
        id: TextInputId
    },
    // TODO: the way we do containers is so shitty
    Container {
        content: Box<Widget<'a>>,
        kind: ContainerKind
    },
    NotYetSupported,
    RichText {
        spans: Vec<Span<'a>>
    },
    Text(&'a str),
    Tag(&'a str),
    Reference(&'a str),
    InlineCode(&'a str),
    #[allow(dead_code)]
    InlineMath(&'a str),
    Error(Cow<'a, str>),
    Heading {
        title: Vec<Widget<'a>>,
        content: Vec<Widget<'a>>,
        nesting: u8
    },
    Column(Spacing, Vec<Widget<'a>>),
    Row(Spacing, Vec<Widget<'a>>),
    Space,
    Separator
}

#[derive(Debug)]
pub enum Spacing {
    None,
    Normal
}

impl From<Spacing> for Pixels {
    fn from(val: Spacing) -> Self {
        match val {
            Spacing::None => 0.0,
            Spacing::Normal => 5.0
        }
        .into()
    }
}

impl From<Spacing> for Padding {
    fn from(val: Spacing) -> Self {
        match val {
            Spacing::None => 0.0,
            Spacing::Normal => 5.0
        }
        .into()
    }
}

impl<'a> Widget<'a> {
    pub fn container(widget: impl Into<Widget<'a>>, kind: ContainerKind) -> Self {
        Self::Container {
            content: Box::new(widget.into()),
            kind
        }
    }

    pub fn row(spacing: Spacing, items: impl IntoIterator<Item = Widget<'a>>) -> Self {
        Self::Row(spacing, items.into_iter().collect())
    }

    pub fn column(spacing: Spacing, items: impl IntoIterator<Item = Widget<'a>>) -> Self {
        Self::Column(spacing, items.into_iter().collect())
    }

    pub fn rich_text(items: impl IntoIterator<Item = Span<'a>>) -> Self {
        Self::RichText {
            spans: items.into_iter().collect()
        }
    }

    const BORDER_WIDTH: f32 = 2.;
    const BORDER_RADIUS: f32 = 4.;
    const SHADOW_BLUR: f32 = 4.;
    const ALPHA: f32 = 0.2;
    const DEFAULT_FONT_SIZE: f32 = 16.;
    pub fn render(self, theme: Theme) -> Element<'a, Message, Theme> {
        match self {
            Widget::TextInput {
                content,
                placeholder,
                id
            } => widget::sensor(
                widget::text_input(placeholder, content)
                    .style(|theme: &Theme, _| text_input::Style {
                        background: Background::Color(theme.crust),
                        border: Border::default(),
                        placeholder: theme.subtext1,
                        value: theme.text,
                        selection: theme.accent.scale_alpha(Self::ALPHA),
                        icon: Color::TRANSPARENT
                    })
                    .id(id)
                    .on_input(move |text| Message::Type(id, text))
                    .on_submit(Message::Submit(id))
                    .padding(0)
            )
            .on_show(move |_| Message::Focus(id.into()))
            .into(),
            Widget::Container { content, kind } => {
                let background = match kind {
                    ContainerKind::BorderedBox | ContainerKind::BorderedBoxFocused => {
                        Some(theme.base)
                    }
                    ContainerKind::Padded => None,
                    ContainerKind::Bar => Some(theme.crust),
                    ContainerKind::Aligned { .. } => None,
                    ContainerKind::Mantle => Some(theme.mantle)
                }
                .map(Background::Color);

                let border = match kind {
                    ContainerKind::BorderedBox => Border::default()
                        .rounded(Self::BORDER_RADIUS)
                        .color(theme.overlay0)
                        .width(Self::BORDER_WIDTH),
                    ContainerKind::BorderedBoxFocused => Border::default()
                        .rounded(Self::BORDER_RADIUS)
                        .color(theme.accent)
                        .width(Self::BORDER_WIDTH),
                    ContainerKind::Padded
                    | ContainerKind::Mantle
                    | ContainerKind::Bar
                    | ContainerKind::Aligned { .. } => Border::default()
                };

                let (height, width, padding) = match kind {
                    ContainerKind::Aligned {
                        horizontal: Some(_),
                        vertical: None
                    }
                    | ContainerKind::Bar => (Shrink, Fill, Spacing::None),
                    ContainerKind::Aligned { .. } => (Fill, Shrink, Spacing::None),
                    _ => (Fill, Fill, Spacing::Normal)
                };

                let shadow = if ContainerKind::BorderedBoxFocused == kind {
                    Shadow {
                        color: theme.crust,
                        blur_radius: Self::SHADOW_BLUR,
                        ..Default::default()
                    }
                } else {
                    Shadow::default()
                };

                let vertical = if let ContainerKind::Aligned {
                    vertical: Some(align),
                    ..
                } = kind
                {
                    align
                } else {
                    Alignment::Start
                };

                let horizontal = if let ContainerKind::Aligned {
                    horizontal: Some(align),
                    ..
                } = kind
                {
                    align
                } else {
                    Alignment::Start
                };

                widget::container(content.render(theme))
                    .padding(padding)
                    .height(height)
                    .width(width)
                    .align_x(horizontal)
                    .align_y(vertical)
                    .style(move |_| container::Style {
                        text_color: None,
                        background,
                        border,
                        shadow,
                        snap: false
                    })
                    .into()
            }
            Widget::NotYetSupported => container("This widget is not yet supported")
                .padding(
                    Padding::default()
                        .left(Spacing::Normal)
                        .right(Spacing::Normal)
                )
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.danger),
                    background: Some(Background::Color(theme.surface0)),
                    border: Border::default()
                        .width(Self::BORDER_WIDTH)
                        .rounded(Self::BORDER_RADIUS)
                        .color(theme.surface1),
                    shadow: Shadow::default(),
                    snap: false
                })
                .into(),
            Widget::Text(text) => text.into(),
            Widget::Tag(text) => widget::container(text)
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.crust),
                    background: Some(Background::Color(theme.accent)),
                    border: Border::default().rounded(Self::BORDER_RADIUS),
                    shadow: Shadow {
                        color: theme.crust,
                        offset: Vector::ZERO,
                        blur_radius: Self::SHADOW_BLUR
                    },
                    snap: false
                })
                .into(),
            Widget::Reference(text) => widget::text(text)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.overlay0)
                })
                .into(),
            Widget::InlineCode(code) => widget::container(widget::text(code).font(Font::MONOSPACE))
                .style(|theme: &Theme| container::Style {
                    text_color: Some(theme.subtext0),
                    background: Some(Background::Color(theme.crust)),
                    border: Border::default().rounded(Self::BORDER_RADIUS),
                    shadow: Shadow {
                        color: theme.crust,
                        offset: Vector::ZERO,
                        blur_radius: Self::SHADOW_BLUR
                    },
                    snap: false
                })
                .into(),
            Widget::Error(text) => widget::text(text)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.danger)
                })
                .into(),
            // TODO: make them collapsable
            Widget::Heading {
                title,
                content,
                nesting
            } => widget::Column::from_iter(
                iter::once(
                    // TODO: If there's for example a row containing RichText then it won't be styled
                    widget::Row::from_iter(title.into_iter().map(|widget| match widget {
                        Widget::RichText { spans } => {
                            Self::render_rich_text(theme, spans, Some(nesting)).into()
                        }
                        _ => widget.render(theme)
                    }))
                    .into()
                )
                .chain(content.into_iter().map(|widget| widget.render(theme)))
            )
            .into(),
            Widget::Column(spacing, widgets) => {
                let elements = widgets.into_iter().map(|widget| widget.render(theme));
                Column::from_iter(elements).spacing(spacing).into()
            }
            Widget::Row(spacing, widgets) => {
                let elements = widgets.into_iter().map(|widget| widget.render(theme));
                Row::from_iter(elements).spacing(spacing).into()
            }
            Widget::Space => widget::space().into(),
            Widget::Separator => widget::rule::horizontal(2.0)
                .style(|theme: &Theme| widget::rule::Style {
                    color: theme.accent,
                    radius: Self::BORDER_RADIUS.into(),
                    fill_mode: FillMode::Full,
                    snap: false
                })
                .into(),
            Widget::RichText { spans } => Self::render_rich_text(theme, spans, None).into(),
            Widget::InlineMath(_) => Self::NotYetSupported.render(theme)
        }
    }

    fn render_rich_text(
        theme: Theme,
        spans: Vec<Span<'_>>,
        heading: Option<u8>
    ) -> widget::text::Rich<'_, std::convert::Infallible, Message, Theme> {
        widget::text::Rich::from_iter(spans.into_iter().map(|span| {
            let mut font = Font::default();
            let modifiers = &span.modifiers;
            if modifiers.contains(Modifiers::BOLD) {
                font.weight = Weight::Bold;
            }

            if modifiers.contains(Modifiers::ITALIC) {
                font.style = font::Style::Italic;
            }

            let bg = if modifiers.contains(Modifiers::HIGHLIGHT) {
                Some(Background::Color(theme.accent.scale_alpha(Self::ALPHA)))
            } else {
                None
            };

            // TODO: make this theme colors
            let fg = if modifiers.contains(Modifiers::STRIKETHROUGH) {
                theme.danger
            } else if modifiers.contains(Modifiers::BOLD | Modifiers::ITALIC) {
                theme.misc_colors[2]
            } else if modifiers.contains(Modifiers::BOLD) {
                theme.misc_colors[0]
            } else if modifiers.contains(Modifiers::ITALIC) {
                theme.misc_colors[1]
            } else if let Some(heading) = heading {
                theme.misc_colors[(heading - 1) as usize]
            } else {
                theme.text
            };

            widget::span(span.text)
                .strikethrough(modifiers.contains(Modifiers::STRIKETHROUGH))
                .background_maybe(bg)
                .color(fg)
                .font(font)
                .border(Border::default().rounded(Self::BORDER_RADIUS))
        }))
        .size(Self::DEFAULT_FONT_SIZE + (6 - heading.unwrap_or(6)) as f32 * 2.)
        .on_link_click(never)
    }
}

impl<'a, T: Into<Widget<'a>>> From<Option<T>> for Widget<'a> {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(Widget::Space)
    }
}

impl<'a> From<&'a str> for Widget<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(value)
    }
}

#[derive(Debug, Clone)]
pub struct Span<'a> {
    pub modifiers: Modifiers,
    pub text: Cow<'a, str>
}

#[macro_export]
macro_rules! row {
    ($spacing:expr, $($x:expr),+ $(,)?) => {
        Widget::row($spacing, [$($x.into()),+])
    };
}

#[macro_export]
macro_rules! column {
    ($spacing:expr, $($x:expr),+ $(,)?) => {
        Widget::column($spacing, [$($x.into()),+])
    };
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerKind {
    BorderedBox,
    BorderedBoxFocused,
    /// This container is meant to be used as a wrapper of spaced elements (such as in the split layout)
    Mantle,
    Padded,
    Aligned {
        horizontal: Option<Alignment>,
        vertical: Option<Alignment>
    },
    Bar
}
