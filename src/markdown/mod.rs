#![allow(dead_code)]
mod store;
mod view;

use itertools::{Itertools, PeekingNext};
use pest::{Parser, Span, iterators::Pair};
use pest_derive::Parser;
use yoke::Yokeable;

pub use crate::markdown::store::MarkdownStore;
use crate::iced_helpers::Modifiers;

// TODO: it would be better to use chumsky as this parser is pretty slow
#[derive(Parser)]
#[grammar = "./markdown/markdown.pest"]
struct MarkdownParser;

#[derive(Debug, Default, Yokeable)]
pub struct Markdown<'a> {
    pub yaml: Option<Yaml<'a>>,
    pub content: Vec<Block<'a>>
}

impl<'a> Markdown<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(input: String) -> MarkdownStore {
        MarkdownStore::new(input)
    }

    fn parse(input: &'a str) -> Self {
        let mut pairs = MarkdownParser::parse(Rule::main, input)
            .expect("Parsing markdown should be infallible")
            .peekable();
        let yaml = pairs
            .next_if(|pair| pair.as_rule() == Rule::frontmatter)
            .map(|pair| Yaml {
                span: pair.as_span(),
                inner_span: pair.into_inner().next().unwrap().as_span()
            });

        let content = pairs.map(Block::parse).collect();

        Self { yaml, content }
    }
}

#[derive(Debug)]
pub struct Yaml<'a> {
    pub inner_span: Span<'a>,
    pub span: Span<'a>
}

#[derive(Debug)]
pub struct Block<'a> {
    pub span: Span<'a>,
    pub kind: BlockKind<'a>
}

#[derive(Debug)]
pub enum BlockKind<'a> {
    Line(Line<'a>),
    Code {
        content: Span<'a>,
        language: Option<Span<'a>>
    },

    ListItem(ListItem<'a>),
    Quote {
        content: Vec<Line<'a>>
    },

    Callout {
        kind: Span<'a>,

        title: Option<Span<'a>>,
        content: Vec<Line<'a>>
    },

    Math {
        inner: Span<'a>
    },

    Heading {
        // Heading title including the #'s
        title_full: Span<'a>,
        nesting: u8,
        title: Line<'a>,
        content: Vec<Block<'a>>
    },

    Ruler,
    Comment {
        inner: Span<'a>
    }
}

#[derive(Debug)]
pub struct ListItem<'a> {
    pub indentation: u16,
    pub content: Line<'a>,
    pub kind: ListItemType,
    pub subitems: Vec<(Span<'a>, ListItem<'a>)>
}

impl<'a> ListItem<'a> {
    fn parse(mut inner: impl PeekingNext<Item = Pair<'a, Rule>>) -> Self {
        let indentation = inner
            .peeking_take_while(|pair| pair.as_rule() == Rule::indentation)
            .count() as u16;
        let kind = inner.next().unwrap();
        let kind = match kind.as_rule() {
            Rule::bullet => ListItemType::Bullet,
            Rule::task_due => ListItemType::Task(false),
            Rule::task_done => ListItemType::Task(true),
            Rule::numbered => {
                let str = &kind.as_str()[..(kind.as_str().len() - 1)];
                ListItemType::Numbered(str.parse().unwrap())
            }
            _ => panic!("Invalid rule in list item kind {kind}")
        };

        let content = Line::parse(inner.next().unwrap().into_inner());
        let subitems = inner
            .map(|pair| (pair.as_span(), Self::parse(pair.into_inner().peekable())))
            .collect();

        Self {
            indentation,
            content,
            subitems,
            kind
        }
    }
}

impl<'a> Block<'a> {
    fn parse(pair: Pair<'a, Rule>) -> Self {
        use BlockKind as B;
        let span = pair.as_span();
        let rule = pair.as_rule();
        let mut inner = pair.into_inner().peekable();
        let kind = match rule {
            Rule::code_block => {
                let pair = inner.next().unwrap();
                match pair.as_rule() {
                    Rule::code_lang => {
                        let language = Some(pair.as_span());
                        let content = inner.next().unwrap().as_span();
                        B::Code { language, content }
                    }
                    Rule::code_block_content => B::Code {
                        language: None,
                        content: pair.as_span()
                    },
                    _ => panic!("Invalid rule in code block {pair}")
                }
            }
            Rule::math_block => B::Math {
                inner: inner.next().unwrap().as_span()
            },
            Rule::callout => B::Callout {
                kind: inner.next().unwrap().as_span(),
                title: inner
                    .next_if(|pair| pair.as_rule() == Rule::callout_title)
                    .map(|pair| pair.as_span()),
                content: inner.map(|pair| Line::parse(pair.into_inner())).collect()
            },
            Rule::quote => B::Quote {
                content: inner.map(|pair| Line::parse(pair.into_inner())).collect()
            },
            Rule::list_item => B::ListItem(ListItem::parse(inner)),
            Rule::heading => {
                let title = inner.next().unwrap();
                let title_full = title.as_span();
                let mut title_inner = title.into_inner();
                B::Heading {
                    nesting: title_inner.next().unwrap().as_str().len() as u8,
                    title_full,
                    title: Line::parse(title_inner.next().unwrap().into_inner()),
                    content: inner.map(Block::parse).collect()
                }
            }
            Rule::line => B::Line(Line::parse(inner)),
            Rule::comment => B::Comment {
                inner: inner.next().unwrap().as_span()
            },
            Rule::ruler => B::Ruler,
            rule => panic!("Invalid rule inside a block: {rule:?}")
        };

        Block { span, kind }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Line<'a>(pub Vec<LineItem<'a>>);
impl<'a> Line<'a> {
    fn parse(pairs: impl Iterator<Item = Pair<'a, Rule>>) -> Self {
        use LineItemKind as I;

        fn modifier_span<'a>(
            modifier: Modifiers,
            pairs: impl Iterator<Item = Pair<'a, Rule>>
        ) -> LineItemKind<'a> {
            I::ModifierSpan(modifier, Line::parse(pairs))
        }

        let items = pairs
            .map(|pair| {
                let span = pair.as_span();
                let rule = pair.as_rule();
                let mut inner = pair.into_inner().peekable();
                let kind = match rule {
                    Rule::bold => modifier_span(Modifiers::BOLD, inner),
                    Rule::italic => modifier_span(Modifiers::ITALIC, inner),
                    Rule::bold_italic => modifier_span(Modifiers::ITALIC | Modifiers::BOLD, inner),
                    Rule::highlight => modifier_span(Modifiers::HIGHLIGHT, inner),
                    Rule::strikethrough => modifier_span(Modifiers::STRIKETHROUGH, inner),
                    Rule::text | Rule::text_wrapped => I::Text,
                    Rule::escaped_char => I::EscapedChar,
                    Rule::inline_code_block => I::InlineCodeBlock {
                        inner: inner.next().unwrap().as_span()
                    },
                    Rule::inline_math_block => I::InlineMathBlock {
                        inner: inner.next().unwrap().as_span()
                    },
                    Rule::link => {
                        let mut target = inner.next().unwrap().into_inner();
                        let file_target = target.next().unwrap().as_span();
                        let subtarget = match target.next() {
                            Some(subtarget) if subtarget.as_rule() == Rule::heading_link => {
                                Subtarget::Heading(subtarget.as_span())
                            }
                            Some(subtarget) if subtarget.as_rule() == Rule::reference_link => {
                                Subtarget::Reference(subtarget.as_span())
                            }
                            Some(rule) => panic!("Invalid rule inside link: {rule}"),
                            _ => Subtarget::None
                        };
                        let display = inner.next().map(|pair| pair.as_span());
                        I::Link {
                            file_target,
                            subtarget,
                            display
                        }
                    }
                    Rule::embed => I::Embed {
                        inner: inner.next().unwrap().as_span()
                    },
                    Rule::tag => I::Tag,
                    Rule::reference => I::Reference,
                    Rule::comment => I::Comment,
                    Rule::external_link => I::ExternalLink {
                        display: inner
                            .next_if(|pair| pair.as_rule() == Rule::external_link_display)
                            .map(|pair| pair.as_span()),
                        target: inner.next().unwrap().as_span()
                    },
                    Rule::external_embed => I::ExternalEmbed {
                        target: inner.next().unwrap().as_span()
                    },
                    Rule::soft_break => I::SoftBreak,
                    rule => panic!("Invalid rule inside line: {rule:?}")
                };
                LineItem { span, kind }
            })
            .collect();
        Self(items)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LineItem<'a> {
    pub span: Span<'a>,
    pub kind: LineItemKind<'a>
}

#[derive(Debug, PartialEq, Eq)]
pub enum LineItemKind<'a> {
    ModifierSpan(Modifiers, Line<'a>),
    Text,
    InlineCodeBlock {
        inner: Span<'a>
    },
    InlineMathBlock {
        inner: Span<'a>
    },
    SoftBreak,
    Link {
        file_target: Span<'a>,
        subtarget: Subtarget<'a>,
        display: Option<Span<'a>>
    },
    ExternalLink {
        target: Span<'a>,
        display: Option<Span<'a>>
    },

    ExternalEmbed {
        target: Span<'a>
    },

    Embed {
        inner: Span<'a>
    },

    EscapedChar,
    Tag,
    Reference,
    Comment
}

#[derive(Debug, PartialEq, Eq)]
pub enum Subtarget<'a> {
    None,
    Heading(Span<'a>),
    Reference(Span<'a>)
}

#[derive(Debug)]
pub enum ListItemType {
    Bullet,
    Numbered(u16),
    Task(bool)
}
