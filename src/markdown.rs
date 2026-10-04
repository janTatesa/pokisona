use std::{borrow::Cow, str::FromStr};

use bitflags::bitflags;
use chumsky::{
    extra::{Full, SimpleState},
    input::MapExtra,
    prelude::*,
    text::newline
};
use serde::{Deserialize, Serialize};

use crate::cache::IdeaRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Markdown(Vec<MarkdownLine>);

impl Markdown {
    pub fn new(source: &str) -> Self {
        let output = MarkdownLine::parser()
            .separated_by(newline())
            .collect()
            .parse(source);
        debug_assert!(!output.has_errors());
        let output = output
            .into_output()
            .expect("Parsing markdown should be infallible");
        Self(output)
    }

    pub fn lines(&self) -> &[MarkdownLine] {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownLine {
    pub list_item: Option<ListItem>,
    pub spans: Vec<MarkdownSpan>
}

impl MarkdownLine {
    pub fn parser<'a>()
    -> impl Parser<'a, &'a str, MarkdownLine, Full<EmptyErr, SimpleState<Modifiers>, ParsingContext>>
    {
        let link = link();
        list_item_start()
            .or_not()
            .then(recursive(|markdown| {
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
                    // HACK: find a better way to do this
                    link.map(|link| vec![link]),
                    just("#")
                        .ignore_then(none_of(" \r\n").repeated().at_least(1).map_with(
                            |(), extra| {
                                let slice: &str = extra.slice();
                                vec![MarkdownSpan::Tag(slice.to_string())]
                            }
                        ))
                        .boxed()
                ))
                .boxed();
                let text = choice((non_text.clone().ignored(), newline()))
                    .not()
                    .then(choice((
                        just("*")
                            .contextual()
                            .configure(|_, ctx: &ParsingContext| {
                                !ctx.contains(ParsingContext::ASTERISK)
                            })
                            .ignored()
                            .boxed(),
                        just("_")
                            .contextual()
                            .configure(|_, ctx: &ParsingContext| {
                                !ctx.contains(ParsingContext::UNDERSCORE)
                            })
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
                    .map_with(|(), extra| {
                        let slice: &str = extra.slice();
                        MarkdownSpan::Text(slice.to_string(), extra.state().0)
                    })
                    .boxed();
                choice((non_text, text.map(|text| vec![text])))
                    .repeated()
                    .collect()
                    .map(|spans: Vec<Vec<MarkdownSpan>>| spans.into_iter().flatten().collect())
            }))
            .boxed()
            .map(|(list_item, spans)| MarkdownLine { list_item, spans })
    }
}

fn link<'a>()
-> Boxed<'a, 'a, &'a str, MarkdownSpan, Full<EmptyErr, SimpleState<Modifiers>, ParsingContext>> {
    choice((just("]]"), just("|")))
        .not()
        .then_ignore(any())
        .repeated()
        .try_map_with(|(), extra| IdeaRef::from_str(extra.slice()).map_err(|_| EmptyErr::default()))
        .then(
            just("|")
                .ignore_then(
                    just("]]")
                        .not()
                        .then(any().repeated().exactly(1))
                        .repeated()
                        .at_least(1)
                        .map_with(|(), extra| {
                            let slice: &str = extra.slice();
                            slice.to_string()
                        })
                )
                .or_not()
                .boxed()
        )
        .delimited_by(just("[["), just("]]"))
        .map_with(
            |(target, display),
             extra: &mut MapExtra<'_, '_, _, Full<_, SimpleState<Modifiers>, _>>| {
                MarkdownSpan::Link {
                    display,
                    target,
                    modifiers: extra.state().0
                }
            }
        )
        .boxed()
}

fn list_item_start<'a>()
-> Boxed<'a, 'a, &'a str, ListItem, Full<EmptyErr, SimpleState<Modifiers>, ParsingContext>> {
    choice((
        choice((just("- "), just("-"))).map(|_| ListItem::Bullet),
        chumsky::text::digits(10)
            .try_map_with(|(), extra| u32::from_str(extra.slice()).map_err(|_| EmptyErr::default()))
            .then_ignore(choice((just("."), just(". "))))
            .map(ListItem::Number)
    ))
    .boxed()
}

fn modifier_span<'a>(
    markdown: impl Parser<
        'a,
        &'a str,
        Vec<MarkdownSpan>,
        Full<EmptyErr, SimpleState<Modifiers>, ParsingContext>
    > + 'a,
    delimeter: &'static str,
    modifiers: Modifiers,
    context: ParsingContext
) -> impl Parser<'a, &'a str, Vec<MarkdownSpan>, Full<EmptyErr, SimpleState<Modifiers>, ParsingContext>>
{
    let delimeter = just(delimeter).map(move |_| MarkdownSpan::ModifierDelimeter(delimeter.into()));
    map_ctx(move |ctx| *ctx | context, markdown)
        .delimited_by(delimeter, delimeter)
        .contextual()
        .configure(move |_, ctx: &ParsingContext| !ctx.contains(context))
        .with_state(SimpleState(modifiers))
        .boxed()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ListItem {
    Bullet,
    Number(u32)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarkdownSpan {
    ModifierDelimeter(Cow<'static, str>),
    Text(String, Modifiers),
    Tag(String),
    Link {
        display: Option<String>,
        target: IdeaRef,
        modifiers: Modifiers
    }
}

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Modifiers: u8 {
        const NONE = 0;
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct ParsingContext: u8 {
        const NONE = 0;
        const ASTERISK = 1 << 0;
        const UNDERSCORE = 1 << 1;
    }
}
