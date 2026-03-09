// Simple incomplete markdown parser for prototyping
use bitflags::bitflags;
use chumsky::{extra::Context, prelude::*};
pub struct Markdown(Vec<MarkdownSpan>);

struct MarkdownSpan {
    kind: MarkdownSpanKind,
    span: SimpleSpan
}

enum MarkdownSpanKind {
    EscapedChar,
    Text,
    ModifierSpan(usize, Modifiers, Vec<MarkdownSpan>),
    Link(Link)
}

pub struct Link {
    pub display: Option<SimpleSpan>,
    pub target: SimpleSpan
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Modifiers: u8 {
        const NONE = 0;
        const BOLD = 1;
        const ITALIC = 1 << 1;
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct ParsingContext: u8 {
        const NONE = 0;
        const ASTERISK = 1;
        const UNDERSCORE = 1 << 1;
    }
}

fn markdown_parser<'a>() -> impl Parser<'a, &'a str, Vec<MarkdownSpan>, Context<ParsingContext>> {
    let link_display = just("|")
        .ignore_then(
            none_of("|")
                .then(any().repeated().exactly(1))
                .repeated()
                .map_with(|_, extra| extra.span())
                .or_not()
        )
        .boxed();
    let link = just("[[")
        .ignore_then(
            choice((just("]]"), just("|")))
                .not()
                .then(any().repeated().exactly(1))
                .repeated()
                .map_with(|_, extra| extra.span())
                .then(link_display)
        )
        .map_with(|(target, display), extra| MarkdownSpan {
            kind: MarkdownSpanKind::Link(Link { display, target }),
            span: extra.span()
        })
        .boxed();
    let escaped_char = just("\\")
        .then(any().repeated().exactly(1))
        .map_with(|_, extra| MarkdownSpan {
            kind: MarkdownSpanKind::EscapedChar,
            span: extra.span()
        })
        .boxed();
    recursive(|markdown| {
        let non_text = choice((
            modifier_span(
                markdown.clone(),
                "***",
                Modifiers::BOLD | Modifiers::ITALIC,
                ParsingContext::ASTERISK
            ),
            modifier_span(
                markdown.clone(),
                "**",
                Modifiers::BOLD,
                ParsingContext::ASTERISK
            ),
            modifier_span(
                markdown.clone(),
                "*",
                Modifiers::ITALIC,
                ParsingContext::ASTERISK
            ),
            modifier_span(
                markdown.clone(),
                "___",
                Modifiers::BOLD | Modifiers::ITALIC,
                ParsingContext::UNDERSCORE
            ),
            modifier_span(
                markdown.clone(),
                "__",
                Modifiers::BOLD,
                ParsingContext::UNDERSCORE
            ),
            modifier_span(
                markdown.clone(),
                "_",
                Modifiers::ITALIC,
                ParsingContext::UNDERSCORE
            ),
            link,
            escaped_char
        ))
        .boxed();
        let text = non_text
            .clone()
            .not()
            .then(choice((
                just("*")
                    .contextual()
                    .configure(|_, ctx: &ParsingContext| !ctx.contains(ParsingContext::ASTERISK))
                    .ignored()
                    .boxed(),
                just("_")
                    .contextual()
                    .configure(|_, ctx: &ParsingContext| !ctx.contains(ParsingContext::UNDERSCORE))
                    .ignored()
                    .boxed(),
                choice((just("*"), just("_")))
                    .not()
                    .then(any().repeated().exactly(1))
                    .ignored()
                    .boxed()
            )))
            .repeated()
            .at_least(1)
            .map_with(|_, extra| MarkdownSpan {
                kind: MarkdownSpanKind::EscapedChar,
                span: extra.span()
            })
            .boxed();
        choice((non_text, text)).repeated().collect()
    })
    .boxed()
}

fn modifier_span<'a>(
    markdown: impl Parser<'a, &'a str, Vec<MarkdownSpan>, Context<ParsingContext>>,
    delimeter: &'static str,
    modifiers: Modifiers,
    context: ParsingContext
) -> impl Parser<'a, &'a str, MarkdownSpan, Context<ParsingContext>> {
    map_ctx(move |ctx| *ctx | context, markdown)
        .delimited_by(just(delimeter), just(delimeter))
        .map_with(move |inner, extra| MarkdownSpan {
            kind: MarkdownSpanKind::ModifierSpan(delimeter.len(), modifiers, inner),
            span: extra.span()
        })
        .contextual()
        .configure(move |_, ctx: &ParsingContext| !ctx.contains(context))
}
