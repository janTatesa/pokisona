use std::{
    iter::{self, Peekable},
    slice
};

use itertools::{Either, Itertools, put_back};

use super::{Block, BlockKind, Line, LineItem, Markdown};
use crate::widget::{Modifiers, Spacing, Span, Widget};
impl<'a> Markdown<'a> {
    pub fn render(&'a self) -> Widget<'a> {
        let iter = self
            .yaml
            .as_ref()
            .map(|_| Widget::NotYetSupported)
            .into_iter()
            .chain(self.content.iter().map(Block::render));
        Widget::column(Spacing::Normal, iter)
    }
}

impl<'a> Block<'a> {
    fn render(&'a self) -> Widget<'a> {
        match &self.kind {
            BlockKind::Line(line) => Widget::row(
                Spacing::Normal,
                LineItemIterWrapper(LineItemIter::new(line, Modifiers::empty()).peekable())
            ),
            BlockKind::Code { .. }
            | BlockKind::ListItem(_)
            | BlockKind::Quote { .. }
            | BlockKind::Callout { .. }
            | BlockKind::Math { .. } => Widget::NotYetSupported,
            BlockKind::Heading {
                nesting,
                title,
                content,
                ..
            } => Widget::Heading {
                title: LineItemIterWrapper(LineItemIter::new(title, Modifiers::empty()).peekable())
                    .collect(),
                content: content.iter().map(Self::render).collect(),
                nesting: *nesting
            },
            BlockKind::Ruler => Widget::Separator,
            BlockKind::Comment { .. } => Widget::Space
        }
    }
}

// TODO: simplify this
// sis literally needed to create two itterators for such a simple task as making modified spans unified in a single rich text
// what's wrong with me
struct LineItemIterWrapper<'a>(Peekable<LineItemIter<'a>>);
impl<'a> Iterator for LineItemIterWrapper<'a> {
    type Item = Widget<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let widget = self.0.next()?.right_or_else(|span| {
            // TODO: When put back is turned to a method, use it instead
            let mut put_back = put_back(&mut self.0);
            let iter = iter::once(span)
                .chain(put_back.peeking_map_while(|item| item.map_right(Either::Right)));
            Widget::rich_text(iter)
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
    type Item = Either<Span<'a>, Widget<'a>>;

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
                    text: line_item.span.as_str().into()
                }));
            }
            I::InlineCodeBlock { inner } => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::CODE,
                    text: inner.as_str().into()
                }));
            }

            I::InlineMathBlock { inner } => Widget::InlineMath(inner.as_str()),
            I::SoftBreak
            | I::EscapedChar
            | I::Link { .. }
            | I::ExternalLink { .. }
            | I::ExternalEmbed { .. }
            | I::Embed { .. } => Widget::NotYetSupported,
            I::Tag => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::TAG,
                    text: line_item.span.as_str().into()
                }));
            }

            I::Reference => {
                return Some(Either::Left(Span {
                    modifiers: self.modifiers | Modifiers::REFERENCE,
                    text: line_item.span.as_str().into()
                }));
            }

            I::Comment => Widget::Space
        };

        Some(Either::Right(widget))
    }
}
