use std::iter::{self};

use iced::{
    Alignment::Start,
    Border, Element, Length,
    font::{self, Weight},
    widget::{
        Column, column, container, rule, space, span, text,
        text::{Rich, Span}
    }
};
use pokisona_markdown::{
    Block, BlockKind, LineItem, LineItemKind, MAX_HEADING_NESTING, Markdown, Modifier
};

use crate::{
    app::{Message, Pokisona},
    color::{ACCENT, CRUST, OVERLAY0}
};

pub fn render_markdown<'a>(markdown: &'a Markdown<'a>) -> Element<'a, Message> {
    let iter = markdown
        .yaml
        .as_ref()
        .map(|yaml| {
            // TODO: render the frontmatter
            container(yaml.inner_span.as_str())
                .style(|_| container::background(CRUST))
                .into()
        })
        .into_iter()
        .chain(markdown.content.iter().map(render_block));
    Column::from_iter(iter).spacing(Pokisona::PADDING).into()
}

const CODE_BLOCK_LEN: u8 = 80;
fn render_block<'a>(block: &'a Block<'a>) -> Element<'a, Message> {
    match &block.kind {
        BlockKind::Line(line) => Rich::from_iter(line.0.iter().flat_map(render_line_item)).into(),
        BlockKind::Code { content, language } => {
            let border = Border::default().rounded(Pokisona::BORDER_RADIUS);
            let lang =
                container(language.map(|lang| text(lang.as_str()).size(Pokisona::SMOL_FONT_SIZE)))
                    .align_right(Length::Fixed(Pokisona::FONT_SIZE * CODE_BLOCK_LEN as f32))
                    .align_y(Start);
            container(column![lang, content.as_str()].padding(Pokisona::PADDING))
                .style(move |_| container::background(CRUST).border(border))
                .into()
        }
        BlockKind::ListItem(_) => "Lists are not yet supported".into(),
        BlockKind::Quote { .. } => "Quotes are not yet supported".into(),
        BlockKind::Callout { .. } => "Callouts are not yet supported".into(),
        BlockKind::Math { .. } => "Math blocks are not yet supported".into(),
        BlockKind::Heading {
            nesting,
            title,
            content,
            ..
        } => Column::from_iter(
            iter::once(
                Rich::from_iter(title.0.iter().flat_map(render_line_item))
                    .size(Pokisona::FONT_SIZE + (MAX_HEADING_NESTING + 1 - nesting) as f32 * 2.)
                    .into()
            )
            .chain(content.iter().map(render_block))
        )
        .into(),
        BlockKind::Ruler => rule::horizontal(Pokisona::BORDER_WIDTH).into(),
        BlockKind::Comment { .. } => space().into()
    }
}

#[allow(
    clippy::large_enum_variant,
    reason = "Span is way more common than modifier"
)]
enum SpanIter<'a> {
    Modifier(Modifier, Box<dyn Iterator<Item = Span<'a>> + 'a>),
    Span(Option<Span<'a>>)
}

impl<'a> Iterator for SpanIter<'a> {
    type Item = Span<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SpanIter::Modifier(modifier, span_iter) => {
                let mut span = span_iter.next()?;
                let mut font = span.font.unwrap_or_default();
                match *modifier {
                    modifier if modifier == Modifier::BOLD | Modifier::ITALIC => {
                        font.weight = Weight::Bold;
                        font.style = font::Style::Italic;
                    }
                    Modifier::BOLD => font.weight = Weight::Bold,
                    Modifier::ITALIC => font.style = font::Style::Italic,
                    Modifier::STRIKETHROUGH => span.strikethrough = true,
                    Modifier::HIGHLIGHT => {
                        span = span
                            .background(ACCENT)
                            .color(CRUST)
                            .border(Border::default().rounded(Pokisona::BORDER_RADIUS))
                    }
                    _ => {}
                }
                Some(span.font(font))
            }
            SpanIter::Span(span) => span.take()
        }
    }
}

fn render_line_item<'a>(item: &'a LineItem<'a>) -> SpanIter<'a> {
    let span = match &item.kind {
        LineItemKind::ModifierSpan(modifier, line) => {
            return SpanIter::Modifier(
                *modifier,
                Box::new(line.0.iter().flat_map(render_line_item))
            );
        }
        LineItemKind::Text => span(item.span.as_str()),
        LineItemKind::InlineCodeBlock { inner } => span(inner.as_str())
            .background(CRUST)
            .border(Border::default().rounded(Pokisona::BORDER_RADIUS)),
        LineItemKind::InlineMathBlock { .. } => span("Math blocks are not yet supported"),
        LineItemKind::Link { .. } => span("Links and embeds are not yet supported"),
        LineItemKind::ExternalLink { .. } => span("Links and embed are not yet supported"),
        LineItemKind::ExternalEmbed { .. } => span("Links and embeds are not yet supported"),
        LineItemKind::Embed { .. } => span("Links and embeds are not yet supported"),
        LineItemKind::EscapedChar => span(&item.span.as_str()[1..=1]),
        LineItemKind::Tag => span(item.span.as_str()).color(ACCENT),
        LineItemKind::Reference => span(item.span.as_str()).color(OVERLAY0),
        LineItemKind::Comment => span(""),
        LineItemKind::SoftBreak => span("\n")
    };

    SpanIter::Span(Some(span))
}
