//! Widgets for pokisona
//! The reason this is necessary is to enforce a consistent UI, the performance cost is worth it.
//! An alternative would be to create fiunctions but that wouldn't be enforced
//! Just like the rest of the project this is shitty and incomplete

use std::borrow::Cow;

use bitflags::bitflags;
use iced::{
    Alignment, Background, Border, Color, Font, Length, Padding, Renderer, Shadow,
    advanced::widget::Text,
    border::Radius,
    font::{self, Weight},
    widget
};
use iced_selection::text::{IntoFragment, Rich};
use url::Url;

use crate::{PathBuf, app::Message, theme::Theme};

pub type Element<'a> = iced::Element<'a, Message, Theme>;
pub const SPACING: f32 = 5.0;
pub const ALPHA: f32 = 0.2;
pub const DEFAULT_FONT_SIZE: f32 = 16.;

pub fn rich_text<'a>(
    theme: Theme,
    spans: impl IntoIterator<Item = Span<'a>>,
    heading: Option<u8>
) -> Element<'a> {
    Rich::from_iter(spans.into_iter().map(|span| {
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
            _ => None
        };

        let fg = match (span.link.as_ref(), heading) {
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
            (_, Some(heading)) => theme.misc_colors[(heading - 1) as usize],
            _ => theme.text
        };

        iced_selection::span(span.text)
            .strikethrough(modifiers.contains(Modifiers::STRIKETHROUGH))
            .background_maybe(bg)
            .color(fg)
            .font(font)
            .link_maybe(span.link)
            .border(Border::default().rounded(BORDER_RADIUS))
    }))
    .size(DEFAULT_FONT_SIZE + (6 - heading.unwrap_or(6)) as f32 * 2.)
    .on_link_click(Message::LinkClick)
    .on_link_hover(Message::Hover)
    .on_hover_lost(Message::HoverEnd)
    .into()
}

pub fn text<'a>(str: impl IntoFragment<'a>, color: Color) -> Text<'a, Theme, Renderer> {
    widget::text(str).style(move |_| widget::text::Style { color: Some(color) })
}

const SHADOW_BLUR: f32 = 2.;
pub fn shadow(theme: Theme) -> Shadow {
    Shadow {
        color: theme.crust,
        blur_radius: SHADOW_BLUR,
        ..Default::default()
    }
}

pub const BORDER_WIDTH: f32 = 2.;
pub const BORDER_RADIUS: f32 = 6.;
#[derive(Clone, Copy)]
pub enum BorderType {
    Focused,
    Normal,
    Invisible,
    TitleBarBottom,
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

    pub fn color(self, color: Color) -> Self {
        let color = Some(color);
        Self { color, ..self }
    }

    pub fn stretched(self) -> Self {
        self.heigth(Length::Fill).width(Length::Fill)
    }

    pub fn padded(self) -> Self {
        let padding = SPACING.into();
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
                    BorderType::TitleBarBottom => {
                        Border::default().rounded(Radius::default().bottom(BORDER_RADIUS))
                    }
                },
                shadow: match val.border {
                    BorderType::None | BorderType::TitleBarBottom => Shadow::default(),
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
// TODO: It looks terrible tbh
#[cfg(debug_assertions)]
pub fn not_yet_supported<'a>() -> Element<'a> {
    widget::container("Not yet supported")
        .style(|theme: &Theme| widget::container::Style {
            text_color: Some(theme.danger),
            background: Some(Background::Color(theme.surface0)),
            border: Border::default()
                .width(BORDER_WIDTH)
                .rounded(BORDER_RADIUS)
                .color(theme.surface1),
            shadow: shadow(*theme),
            snap: false
        })
        .into()
}

#[derive(Debug, Clone)]
pub struct Span<'a> {
    pub modifiers: Modifiers,
    pub link: Option<Link>,
    pub text: Cow<'a, str>
}

#[derive(Debug, Clone)]
pub enum Link {
    InvalidUrlExternal(String),
    External(Url),

    Internal(PathBuf),
    /// Currently handled the same as [`Link::InvalidUrlExternal`], in future will create a new file on click
    NonExistentInternal(PathBuf)
}

// TODO: Bitflags make me do long if else chains instead of match arms, maybe a struct of bools would be better
bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct Modifiers: u8 {
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
        const HIGHLIGHT = 1 << 2;
        const STRIKETHROUGH = 1 << 3;
        const CODE = 1 << 4;
        const TAG = 1 << 5;
        const REFERENCE = 1 << 7;
    }
}
