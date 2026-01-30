// Wtf is this shit. Fuck you Tatesa
use std::{
    iter::{self, Enumerate, Peekable},
    rc::Rc,
    slice
};

use iced::{
    Length,
    widget::{
        self, Column,
        scrollable::{Direction, Scrollbar},
        space
    }
};
use itertools::{Either, chain};

use super::iced_helpers::{BORDER_WIDTH, Span, not_yet_supported, rich_text, span};
use crate::{
    Element, Link,
    file_store::FileData,
    markdown::{Block, BlockKind, ListItemType, Markdown, Modifiers, ParagraphItem},
    view::{
        Theme,
        iced_helpers::{SPACING, text}
    }
};

impl<'a> Markdown<'a> {
    pub fn view(&'a self, theme: Theme) -> Element<'a> {
        let iter = self
            .yaml
            .as_ref()
            .map(|_| not_yet_supported("yaml frontmatters").into_element(theme, None))
            .into_iter()
            .chain(self.content.iter().map(|block| block.render(theme)));
        const SCROLLBAR_WIDTH: f32 = BORDER_WIDTH * 2.;
        let scrollbar = Scrollbar::new()
            .width(SCROLLBAR_WIDTH)
            .scroller_width(SCROLLBAR_WIDTH);
        let content = Column::from_iter(iter).spacing(SPACING);
        widget::scrollable(content)
            .direction(Direction::Both {
                vertical: scrollbar,
                horizontal: scrollbar
            })
            .auto_scroll(true)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> Block<'a> {
    fn render(&'a self, theme: Theme) -> Element<'a> {
        const BULLET: &str = "\u{2022}";
        match &self.kind {
            BlockKind::Paragraph(line) => rich_text(
                theme,
                ParagraphItemIter::new(theme, line, Modifiers::empty()),
                None
            ),
            BlockKind::ListItem(item) => {
                let beginning = match item.kind {
                    ListItemType::Bullet => {
                        Either::Left(iter::once(text(BULLET, theme.accent).into()))
                    }
                    ListItemType::Numbered(num) => {
                        Either::Left(iter::once(text(num, theme.accent).into()))
                    }
                    ListItemType::Task(checked) => Either::Right([
                        text(BULLET, theme.accent).into(),
                        widget::checkbox(checked).into()
                    ])
                }
                .into_iter();
                let content = ParagraphItemIter::new(theme, &item.content, Modifiers::empty());
                let content = rich_text(theme, content, None);
                widget::row(chain![beginning, iter::once(content)]).into()
            }
            BlockKind::Code { .. }
            | BlockKind::Quote { .. }
            | BlockKind::Callout { .. }
            | BlockKind::Math { .. } => {
                not_yet_supported("math blocks, code blocks, quotes, callouts")
                    .into_element(theme, None)
            }
            // TODO: make them collapsable
            BlockKind::Heading {
                nesting,
                title,
                content,
                ..
            } => widget::column(
                iter::once(rich_text(
                    theme,
                    ParagraphItemIter::new(theme, title, Modifiers::empty()),
                    Some(*nesting)
                ))
                .chain(content.iter().map(|block| block.render(theme)))
            )
            .into(),
            BlockKind::Ruler => widget::rule::horizontal(BORDER_WIDTH).into(),
            BlockKind::Comment { .. } => space().into()
        }
    }
}

struct ParagraphItemIter<'a> {
    theme: Theme,
    modifiers: Modifiers,
    // TODO: actually display those embeds
    embeds: Vec<Rc<FileData>>,
    inner: Peekable<Enumerate<slice::Iter<'a, ParagraphItem<'a>>>>,
    nested: Option<Box<ParagraphItemIter<'a>>>
}

impl<'a> ParagraphItemIter<'a> {
    fn new(theme: Theme, inner: &'a [ParagraphItem<'a>], modifiers: Modifiers) -> Self {
        Self {
            inner: inner.iter().enumerate().peekable(),
            nested: None,
            modifiers,
            theme,
            embeds: vec![]
        }
    }
}

impl<'a> Iterator for ParagraphItemIter<'a> {
    type Item = Span<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use crate::markdown::ParagraphItemKind as I;
        if let Some(iter) = self.nested.as_mut()
            && let Some(widget) = iter.next()
        {
            return Some(widget);
        }

        let (count, line_item) = self.inner.next()?;
        let content = line_item.span.as_str();
        let modifiers = match &line_item.kind {
            I::ModifierSpan(modifiers, line) => {
                self.nested = Some(Box::new(Self::new(
                    self.theme,
                    line,
                    self.modifiers | *modifiers
                )));
                return self.next();
            }
            I::Text => span(content),
            I::SoftBreak => span("\\\n"),
            I::EscapedChar => span(&line_item.span.as_str()[1..]),
            I::InlineCodeBlock { inner } => span(*inner).modifiers(Modifiers::CODE),
            I::InlineMathBlock { .. } => not_yet_supported("math blocks, embeds"),
            I::ExternalEmbed { target, display } => {
                self.embeds.push(target.clone());
                span(match (display, count == 0 && self.inner.peek().is_none()) {
                    (Some(display), _) => display,
                    (None, false) => target.locator().as_str(),
                    _ => return None
                })
                .link(Link::External(target.locator().clone()))
            }
            I::Embed {
                target,
                display,
                target_str,
                ..
            } => {
                self.embeds.push(target.clone());
                span(match (display, count == 0 && self.inner.peek().is_none()) {
                    (Some(display), _) => display,
                    (None, false) => *target_str,
                    _ => return None
                })
                .link(Link::External(target.locator().clone()))
            }
            I::Tag => span(content).modifiers(Modifiers::TAG),
            I::Reference => span(content).modifiers(Modifiers::REFERENCE),
            I::Link {
                target,
                display,
                target_str,
                ..
            } => {
                let mut path = target.clone();
                if path.extension().is_none() {
                    path.set_extension("md");
                }

                let link = if path.exists() {
                    Link::Internal(path)
                } else {
                    Link::NonExistentInternal(path)
                };

                span(display.unwrap_or(target_str)).link(link)
            }
            I::ExternalLink {
                target, display, ..
            } => {
                let link = Link::External(target.as_str().parse().unwrap());
                span(display.unwrap_or(target.as_str())).link(link)
            }
            I::Comment => span("")
        }
        .modifiers(self.modifiers);
        Some(modifiers)
    }
}
