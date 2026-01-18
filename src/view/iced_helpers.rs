use std::iter::{self, Peekable};

use iced::{
    Alignment, Background, Border, Color, Font, Length, Padding, Renderer, Shadow,
    advanced::widget::Text,
    border::Radius,
    font::{self, Weight},
    widget
};
use iced_selection::text::{Fragment, IntoFragment, Rich};
use lucide_icons::Icon;

use crate::{Element, Link, Message, command::Command, markdown::Modifiers, view::Theme};

pub const SPACING_AND_PADDING: f32 = 5.0;
pub const ALPHA: f32 = 0.2;
pub const DEFAULT_FONT_SIZE: f32 = 16.;

struct SpanIter<I: Iterator> {
    previous_with_bg: bool,
    theme: Theme,
    heading: Option<u8>,
    spans: Peekable<I>
}

impl<'a, I: Iterator<Item = Span<'a>>> Iterator for SpanIter<I> {
    type Item = iced_selection::text::Span<'a, Link>;

    fn next(&mut self) -> Option<Self::Item> {
        let span = self.spans.next()?;
        let theme = self.theme;
        let mut font = Font::default();
        let modifiers = &span.modifiers;
        if modifiers.contains(Modifiers::BOLD) {
            font.weight = Weight::Bold;
        }

        if modifiers.contains(Modifiers::ITALIC) {
            font.style = font::Style::Italic;
        }

        let bg = match () {
            _ if modifiers.contains(Modifiers::TAG) => Some(theme.accent),
            _ if modifiers.contains(Modifiers::CODE) => Some(theme.crust),
            _ if modifiers.contains(Modifiers::HIGHLIGHT) => Some(theme.accent.scale_alpha(ALPHA)),
            _ if modifiers.contains(Modifiers::UNSUPPORTED) => Some(theme.surface0),
            _ => None
        };

        let fg = match (span.link.as_ref(), self.heading) {
            (Some(Link::Internal(_)), _) => theme.link_internal,
            (Some(Link::External(_)), _) => theme.link_external,
            (Some(Link::NonExistentInternal(_) | Link::InvalidUrlExternal(_)), _) => theme.danger,
            _ if modifiers.contains(Modifiers::TAG) => theme.crust,
            _ if modifiers.contains(Modifiers::CODE) => theme.subtext1,
            _ if modifiers.contains(Modifiers::REFERENCE) => theme.overlay0,
            _ if modifiers.contains(Modifiers::STRIKETHROUGH) => theme.danger,
            _ if modifiers.contains(Modifiers::BOLD | Modifiers::ITALIC) => theme.bold_italic,
            _ if modifiers.contains(Modifiers::BOLD) => theme.bold,
            _ if modifiers.contains(Modifiers::ITALIC) => theme.italic,
            (_, Some(heading)) => theme.rainbow[(heading - 1) as usize],
            _ => theme.text
        };

        let has_bg =
            Modifiers::TAG | Modifiers::CODE | Modifiers::HIGHLIGHT | Modifiers::UNSUPPORTED;
        let next_with_bg = self
            .spans
            .peek()
            .is_some_and(|span| span.modifiers.intersects(has_bg));

        let radius = match (self.previous_with_bg, bg.is_some(), next_with_bg) {
            (true, true, false) => Radius::default().right(BORDER_RADIUS),
            (false, true, true) => Radius::default().left(BORDER_RADIUS),
            (false, true, false) => BORDER_RADIUS.into(),
            _ => Default::default()
        };

        self.previous_with_bg = bg.is_some();

        let border = iced_selection::span(span.text)
            .strikethrough(modifiers.contains(Modifiers::STRIKETHROUGH))
            .background_maybe(bg)
            .color(fg)
            .font(font)
            .link_maybe(span.link)
            .border(Border::default().rounded(radius));
        Some(border)
    }
}

pub fn rich_text<'a>(
    theme: Theme,
    spans: impl IntoIterator<Item = Span<'a>>,
    heading: Option<u8>
) -> Element<'a> {
    Rich::from_iter(SpanIter {
        previous_with_bg: false,
        theme,
        heading,
        spans: spans.into_iter().peekable()
    })
    .size(DEFAULT_FONT_SIZE + (6 - heading.unwrap_or(6)) as f32 * 2.)
    .on_link_click(|link| Command::Follow(link).into())
    .on_link_hover(Message::Hover)
    .on_hover_lost(Message::HoverEnd)
    .into()
}

pub fn text<'a>(str: impl IntoFragment<'a>, color: Color) -> Text<'a, Theme, Renderer> {
    widget::text(str).color(color)
}

const SHADOW_BLUR: f32 = 4.;
pub fn shadow(theme: Theme) -> Shadow {
    Shadow {
        color: theme.crust,
        blur_radius: SHADOW_BLUR,
        ..Default::default()
    }
}

pub const BORDER_WIDTH: f32 = 2.;
pub const BORDER_RADIUS: f32 = 3.;
#[derive(Clone, Copy)]
pub enum BorderType {
    Focused,
    Normal,
    Invisible,
    HoveredLinkTitle,
    None
}

pub struct Container<'a> {
    inner: Element<'a>,
    align_x: Alignment,
    align_y: Alignment,
    height: Length,
    width: Length,
    border: BorderType,
    color: Option<Color>,
    padding: Padding
}

impl Container<'_> {
    pub fn align_x(self, align_x: Alignment) -> Self {
        Self { align_x, ..self }
    }

    pub fn align_y(self, align_y: Alignment) -> Self {
        Self { align_y, ..self }
    }

    pub fn heigth(self, height: Length) -> Self {
        Self { height, ..self }
    }

    pub fn width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    pub fn border(self, border: BorderType) -> Self {
        Self { border, ..self }
    }

    pub fn background(self, color: Color) -> Self {
        let color = Some(color);
        Self { color, ..self }
    }

    pub fn stretched(self) -> Self {
        self.heigth(Length::Fill).width(Length::Fill)
    }

    pub fn padded(self) -> Self {
        let padding = SPACING_AND_PADDING.into();
        Self { padding, ..self }
    }

    pub fn custom_padding(self, padding: impl Into<Padding>) -> Self {
        Self {
            padding: padding.into(),
            ..self
        }
    }
}

impl<'a> From<Container<'a>> for Element<'a> {
    fn from(val: Container<'a>) -> Self {
        widget::container(val.inner)
            .style(move |theme: &Theme| widget::container::Style {
                text_color: None,
                background: val.color.map(Background::Color),
                border: match val.border {
                    BorderType::Focused => Border {
                        color: theme.accent,
                        width: BORDER_WIDTH,
                        radius: BORDER_RADIUS.into()
                    },
                    BorderType::Normal => Border {
                        color: theme.overlay0,
                        width: BORDER_WIDTH,
                        radius: BORDER_RADIUS.into()
                    },
                    BorderType::Invisible => Border::default().rounded(BORDER_RADIUS),
                    BorderType::None => Border::default(),
                    BorderType::HoveredLinkTitle => {
                        Border::default().rounded(Radius::default().bottom(BORDER_RADIUS))
                    }
                },
                shadow: match val.border {
                    BorderType::None | BorderType::HoveredLinkTitle => Shadow::default(),
                    _ => shadow(*theme)
                },
                snap: false
            })
            .align_x(val.align_x)
            .align_y(val.align_y)
            .width(val.width)
            .height(val.height)
            .padding(val.padding)
            .into()
    }
}

/// A wrapper around iced's container that provides some convienence emethods
pub fn container<'a>(content: impl Into<Element<'a>>) -> Container<'a> {
    Container {
        inner: content.into(),
        align_x: Alignment::Start,
        align_y: Alignment::Start,
        height: Length::Shrink,
        width: Length::Shrink,
        border: BorderType::None,
        color: None,
        padding: Padding::default()
    }
}

/// An indicator that the ui element is not yet supported. Useful for prototyping
/// A production release shouldn't have UI elements that are not yet implemented so it's enabled only for debug mode
#[cfg(debug_assertions)]
pub fn not_yet_supported(item: &'static str) -> Span<'static> {
    Span {
        modifiers: Modifiers::UNSUPPORTED,
        link: None,
        text: format!("Rendering {item} is not yet supported.").into()
    }
}

#[derive(Debug, Clone)]
pub struct Span<'a> {
    modifiers: Modifiers,
    link: Option<Link>,
    text: Fragment<'a>
}

impl<'a> Span<'a> {
    pub fn modifiers(self, modifiers: Modifiers) -> Self {
        let modifiers = modifiers | self.modifiers;
        Self { modifiers, ..self }
    }

    pub fn link(self, link: Link) -> Self {
        let link = Some(link);
        Self { link, ..self }
    }

    pub fn into_element(self, theme: Theme, heading: Option<u8>) -> Element<'a> {
        rich_text(theme, iter::once(self), heading)
    }
}

pub fn span<'a>(content: impl IntoFragment<'a>) -> Span<'a> {
    Span {
        modifiers: Modifiers::empty(),
        link: None,
        text: content.into_fragment()
    }
}

impl Command {
    fn icon(&self) -> Icon {
        match self {
            Command::Quit(_) => Icon::CircleX,
            Command::VSplit { .. } => Icon::SquareSplitHorizontal,
            Command::HSplit { .. } => Icon::SquareSplitVertical,
            Command::FileHistoryForward => Icon::ArrowRight,
            Command::FileHistoryBackward => Icon::ArrowLeft,
            _ => todo!()
        }
    }
}

pub fn button<'a>(command: Command, color: Color) -> Element<'a> {
    let content = text(command.icon().unicode(), color).font(Font::with_name("lucide"));
    widget::button(content)
        .padding(0)
        .on_press(command.into())
        .into()
}

pub fn button_enabled_if<'a>(
    command: Command,
    color: Color,
    disabled_color: Color,
    condition: bool
) -> Element<'a> {
    let color = if condition { color } else { disabled_color };
    let content = text(command.icon().unicode(), color).font(Font::with_name("lucide"));
    widget::button(content)
        .on_press_maybe(condition.then_some(command.into()))
        .padding(0)
        .into()
}
