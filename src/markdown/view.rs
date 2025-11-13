use std::{
    iter::{self, Peekable},
    slice,
    str::FromStr
};

use iced::widget::{self, Column, space};
use itertools::{Either, chain};
use url::Url;

use super::{Block, BlockKind, Markdown, ParagraphItem};
use crate::{
    PathBuf,
    iced_helpers::{
        BORDER_WIDTH, Element, Link, Modifiers, SPACING, Span, not_yet_supported, rich_text, span
    },
    markdown::ListItemType,
    theme::Theme
};
impl<'a> Markdown<'a> {
    pub fn render(&'a self, theme: Theme) -> Element<'a> {
        let iter = dbg!(self)
            .yaml
            .as_ref()
            .map(|_| not_yet_supported("yaml frontmatters").into_element(theme, None))
            .into_iter()
            .chain(self.content.iter().map(|block| block.render(theme)));
        Column::from_iter(iter).spacing(SPACING).into()
    }
}

impl<'a> Block<'a> {
    fn render(&'a self, theme: Theme) -> Element<'a> {
        const BULLET: &str = "\u{2022}";
        match &self.kind {
            BlockKind::Paragraph(line) => rich_text(
                theme,
                ParagraphItemIter::new(line, Modifiers::empty()),
                None
            ),
            BlockKind::ListItem(item) => {
                let beginning = match item.kind {
                    ListItemType::Bullet => {
                        Either::Left(iter::once(widget::text(BULLET).color(theme.accent).into()))
                    }
                    ListItemType::Numbered(num) => {
                        Either::Left(iter::once(widget::text(num).color(theme.accent).into()))
                    }
                    ListItemType::Task(checked) => Either::Right([
                        widget::text(BULLET).color(theme.accent).into(),
                        widget::checkbox("", checked).into()
                    ])
                }
                .into_iter();
                let content = ParagraphItemIter::new(&item.content, Modifiers::empty());
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
                    ParagraphItemIter::new(title, Modifiers::empty()),
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
    modifiers: Modifiers,
    inner: Peekable<slice::Iter<'a, ParagraphItem<'a>>>,
    nested: Option<Box<ParagraphItemIter<'a>>>
}

impl<'a> ParagraphItemIter<'a> {
    fn new(inner: &'a [ParagraphItem<'a>], modifiers: Modifiers) -> Self {
        Self {
            inner: inner.iter().peekable(),
            nested: None,
            modifiers
        }
    }
}

impl<'a> Iterator for ParagraphItemIter<'a> {
    type Item = Span<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use super::ParagraphItemKind as I;
        if let Some(iter) = self.nested.as_mut()
            && let Some(widget) = iter.next()
        {
            return Some(widget);
        }

        let line_item = self.inner.next()?;
        let content = line_item.span.as_str();
        let modifiers = match &line_item.kind {
            I::ModifierSpan(modifiers, line) => {
                self.nested = Some(Box::new(Self::new(line, self.modifiers | *modifiers)));
                return self.next();
            }
            I::Text => span(content),
            I::SoftBreak => span("\\\n"),
            I::EscapedChar => span(&line_item.span.as_str()[1..]),
            I::InlineCodeBlock { inner } => span(inner.as_str()).modifiers(Modifiers::CODE),
            I::InlineMathBlock { .. } | I::ExternalEmbed { .. } | I::Embed { .. } => {
                not_yet_supported("math blocks, embeds")
            }
            I::Tag => span(content).modifiers(Modifiers::TAG),
            I::Reference => span(content).modifiers(Modifiers::REFERENCE),
            I::Link {
                file_target,
                display,
                ..
            } => {
                let mut path = PathBuf::from(file_target.as_str());
                // TODO: Not robust enough
                // IO should be async for example
                if path.extension().is_none() {
                    path.set_extension("md");
                }

                let link = if path.exists() {
                    Link::Internal(path)
                } else {
                    Link::NonExistentInternal(path)
                };

                span(display.unwrap_or(*file_target).as_str()).link(link)
            }
            I::ExternalLink {
                target, display, ..
            } => {
                let link = Url::from_str(target.as_str())
                    .map(Link::External)
                    .unwrap_or(Link::InvalidUrlExternal(target.as_str().to_string()));
                span(display.unwrap_or(*target).as_str()).link(link)
            }
            I::Comment => span("")
        }
        .modifiers(self.modifiers);
        Some(modifiers)
    }
}
