use std::{
    iter::{self, Peekable},
    slice,
    str::FromStr
};

use iced::widget::{self, Column, space};
use itertools::{Either, Itertools, put_back};
use url::Url;

use super::{Block, BlockKind, Line, LineItem, Markdown};
use crate::{
    PathBuf,
    iced_helpers::{
        BORDER_WIDTH, Element, Link, Modifiers, SPACING, Span, not_yet_supported, rich_text
    },
    theme::Theme
};
impl<'a> Markdown<'a> {
    pub fn render(&'a self, theme: Theme) -> Element<'a> {
        let iter = self
            .yaml
            .as_ref()
            .map(|_| not_yet_supported())
            .into_iter()
            .chain(self.content.iter().map(|block| block.render(theme)));
        Column::from_iter(iter).spacing(SPACING).into()
    }
}

impl<'a> Block<'a> {
    fn render(&'a self, theme: Theme) -> Element<'a> {
        match &self.kind {
            BlockKind::Line(line) => widget::row(LineItemIterWrapper {
                inner: LineItemIter::new(line, Modifiers::empty()).peekable(),
                heading: None,
                theme
            })
            .spacing(SPACING)
            .into(),
            BlockKind::Code { .. }
            | BlockKind::ListItem(_)
            | BlockKind::Quote { .. }
            | BlockKind::Callout { .. }
            | BlockKind::Math { .. } => not_yet_supported(),
            BlockKind::Heading {
                nesting,
                title,
                content,
                ..
            } => widget::column(
                iter::once(
                    widget::row(LineItemIterWrapper {
                        inner: LineItemIter::new(title, Modifiers::empty()).peekable(),
                        heading: Some(*nesting),
                        theme
                    })
                    .into()
                )
                .chain(content.iter().map(|block| block.render(theme)))
            )
            .into(),
            BlockKind::Ruler => widget::rule::horizontal(BORDER_WIDTH).into(),
            BlockKind::Comment { .. } => space().into()
        }
    }
}

// TODO: simplify this
// sis literally needed to create two itterators for such a simple task as making modified spans unified in a single rich text, and even that's incomplete
// what's wrong with me
struct LineItemIterWrapper<'a> {
    inner: Peekable<LineItemIter<'a>>,
    heading: Option<u8>,
    theme: Theme
}

impl<'a> Iterator for LineItemIterWrapper<'a> {
    type Item = Element<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let widget = self.inner.next()?.right_or_else(|span| {
            // TODO: When put back is turned to a method, use it instead
            let mut put_back = put_back(&mut self.inner);
            let iter = iter::once(span)
                .chain(put_back.peeking_map_while(|item| item.map_right(Either::Right)));
            rich_text(self.theme, iter, self.heading)
        });

        Some(widget)
    }
}

struct LineItemIter<'a> {
    modifiers: Modifiers,
    inner: Peekable<slice::Iter<'a, LineItem<'a>>>,
    nested: Option<Box<LineItemIter<'a>>>
}

impl<'a> LineItemIter<'a> {
    fn new(line: &'a Line<'a>, modifiers: Modifiers) -> Self {
        Self {
            inner: line.0.iter().peekable(),
            nested: None,
            modifiers
        }
    }
}

impl<'a> Iterator for LineItemIter<'a> {
    type Item = Either<Span<'a>, Element<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        use super::LineItemKind as I;
        if let Some(widget) = self.nested.as_mut().and_then(|iter| iter.next()) {
            return Some(widget);
        }

        let line_item = self.inner.next()?;
        let widget = match &line_item.kind {
            I::ModifierSpan(modifiers, line) => {
                self.nested = Some(Box::new(Self::new(line, self.modifiers | *modifiers)));
                return self.next();
            }
            I::Text => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers,
                    text: line_item.span.as_str().into(),
                    link: None
                }));
            }
            I::InlineCodeBlock { inner } => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::CODE,
                    text: inner.as_str().into(),
                    link: None
                }));
            }

            I::InlineMathBlock { .. }
            | I::SoftBreak
            | I::EscapedChar
            | I::ExternalEmbed { .. }
            | I::Embed { .. } => not_yet_supported(),
            I::Tag => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::TAG,
                    text: line_item.span.as_str().into(),
                    link: None
                }));
            }

            I::Reference => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::REFERENCE,
                    text: line_item.span.as_str().into(),
                    link: None
                }));
            }
            I::Link {
                file_target,
                display,
                ..
            } => {
                let path = PathBuf::from(file_target.as_str());
                let link = if path.exists() {
                    Link::Internal(path)
                } else {
                    Link::NonExistentInternal(path)
                };

                return Some(Either::Left(Span {
                    modifiers: self.modifiers,
                    text: display.unwrap_or(*file_target).as_str().into(),
                    // TODO: All IO should be async
                    link: Some(link)
                }));
            }
            I::ExternalLink {
                target, display, ..
            } => {
                let link = Url::from_str(target.as_str())
                    .map(Link::External)
                    .unwrap_or(Link::InvalidUrlExternal(target.as_str().to_string()));
                return Some(Either::Left(Span {
                    modifiers: self.modifiers,
                    text: display.unwrap_or(*target).as_str().into(),
                    link: Some(link)
                }));
            }
            I::Comment => space().into()
        };

        Some(Either::Right(widget))
    }
}
