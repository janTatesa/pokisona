// I hate myself for this mess I've created
#![allow(dead_code)]
pub mod store;

use std::rc::Rc;

use bitflags::bitflags;
use iced::Task;
use itertools::{Itertools, PeekingNext};
use pest::{Parser, Span, iterators::Pair};
use pest_derive::Parser;

use crate::{
    Message, PathBuf,
    file_store::{FileData, FileLocator, FileStore}
};

// TODO: it would be better to use chumsky because this is boilerplateous
#[derive(Parser)]
#[grammar = "./markdown.pest"]
struct MarkdownParser;

#[derive(Debug, Default)]
pub struct Markdown<'a> {
    pub yaml: Option<Yaml<'a>>,
    pub content: Vec<Block<'a>>
}

impl<'a> Markdown<'a> {
    fn parse(input: &'a str, file_store: &'static FileStore) -> (Self, Task<Message>) {
        let mut pairs = MarkdownParser::parse(Rule::main, input)
            .expect("Parsing markdown should be infallible")
            .peekable();
        let yaml = pairs
            .next_if(|pair| pair.as_rule() == Rule::frontmatter)
            .map(|pair| Yaml {
                span: pair.as_span(),
                inner_span: pair.into_inner().next().unwrap().as_span()
            });

        let mut tasks = Vec::new();
        let content = pairs
            .map(|pair| Block::parse(pair, file_store, &mut tasks))
            .collect();

        (Self { yaml, content }, Task::batch(tasks))
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
    Paragraph(Vec<ParagraphItem<'a>>),
    Code {
        content: Span<'a>,
        language: Option<Span<'a>>
    },

    ListItem(ListItem<'a>),
    Quote {
        content: Vec<ParagraphItem<'a>>
    },

    Callout {
        kind: Span<'a>,
        title: Option<Span<'a>>,
        content: Vec<ParagraphItem<'a>>
    },

    Math {
        inner: Span<'a>
    },

    Heading {
        // Heading title including the #'s
        title_full: Span<'a>,
        nesting: u8,
        title: Vec<ParagraphItem<'a>>,
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
    pub content: Vec<ParagraphItem<'a>>,
    pub kind: ListItemType,
    pub subitems: Vec<(Span<'a>, ListItem<'a>)>
}

impl<'a> ListItem<'a> {
    fn parse(
        mut pairs: impl PeekingNext<Item = Pair<'a, Rule>>,
        file_store: &'static FileStore,
        tasks: &mut Vec<Task<Message>>
    ) -> Self {
        let indentation = pairs
            .peeking_take_while(|pair| pair.as_rule() == Rule::indentation)
            .count() as u16;
        let kind = pairs.next().unwrap();
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

        let content = pairs.next().unwrap();
        let content = content
            .into_inner()
            .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
            .collect();

        let subitems = pairs
            .map(|pair| {
                (
                    pair.as_span(),
                    Self::parse(pair.into_inner().peekable(), file_store, tasks)
                )
            })
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
    fn parse(
        pair: Pair<'a, Rule>,
        file_store: &'static FileStore,
        tasks: &mut Vec<Task<Message>>
    ) -> Self {
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
                content: inner
                    .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                    .collect()
            },
            Rule::quote => B::Quote {
                content: inner
                    .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                    .collect()
            },
            Rule::list_item => B::ListItem(ListItem::parse(inner, file_store, tasks)),
            Rule::heading => {
                let title = inner.next().unwrap();
                let title_full = title.as_span();
                let mut title_inner = title.into_inner();
                B::Heading {
                    nesting: title_inner.next().unwrap().as_str().len() as u8,
                    title_full,
                    title: title_inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect(),
                    content: inner
                        .map(|pair| Block::parse(pair, file_store, tasks))
                        .collect()
                }
            }
            Rule::paragraph => B::Paragraph(
                inner
                    .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                    .collect()
            ),
            Rule::comment => B::Comment {
                inner: inner.next().unwrap().as_span()
            },
            Rule::ruler => B::Ruler,
            rule => panic!("Invalid rule inside a block: {rule:?}")
        };

        Block { span, kind }
    }
}
impl<'a> ParagraphItem<'a> {
    fn parse(
        pair: Pair<'a, Rule>,
        file_store: &'static FileStore,
        tasks: &mut Vec<Task<Message>>
    ) -> Self {
        use ParagraphItemKind as I;

        let span = pair.as_span();
        let rule = pair.as_rule();
        let mut inner = pair.into_inner().peekable();
        let kind = match rule {
            Rule::bold => {
                let modifier = Modifiers::BOLD;
                I::ModifierSpan(
                    modifier,
                    inner
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect()
                )
            }
            Rule::italic => {
                let modifier = Modifiers::ITALIC;
                I::ModifierSpan(
                    modifier,
                    inner
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect()
                )
            }
            Rule::bold_italic => {
                let modifier = Modifiers::ITALIC | Modifiers::BOLD;
                I::ModifierSpan(
                    modifier,
                    inner
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect()
                )
            }
            Rule::highlight => {
                let modifier = Modifiers::HIGHLIGHT;
                I::ModifierSpan(
                    modifier,
                    inner
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect()
                )
            }
            Rule::strikethrough => {
                let modifier = Modifiers::STRIKETHROUGH;
                I::ModifierSpan(
                    modifier,
                    inner
                        .map(|pair| ParagraphItem::parse(pair, file_store, tasks))
                        .collect()
                )
            }
            Rule::paragraph_text | Rule::line_text | Rule::text_wrapped => I::Text,
            Rule::escaped_char => I::EscapedChar,
            Rule::inline_code_block => I::InlineCodeBlock {
                inner: inner.next().unwrap().as_str()
            },
            Rule::inline_math_block => I::InlineMathBlock {
                inner: inner.next().unwrap().as_str()
            },
            Rule::link => {
                let target = inner.next().unwrap();
                let target_str = target.as_str();
                let mut target = target.into_inner();
                I::Link {
                    target: target.next().unwrap().as_str().into(),
                    subtarget: match target.next() {
                        Some(subtarget) if subtarget.as_rule() == Rule::heading_link => {
                            Subtarget::Heading(subtarget.as_str())
                        }
                        Some(subtarget) if subtarget.as_rule() == Rule::reference_link => {
                            Subtarget::Reference(subtarget.as_str())
                        }
                        Some(rule) => panic!("Invalid rule inside link: {rule}"),
                        _ => Subtarget::None
                    },
                    display: inner.next().map(|pair| pair.as_str()),
                    target_str
                }
            }
            Rule::embed => {
                let target = inner.next().unwrap();
                let target_str = target.as_str();
                let mut target = target.into_inner();
                I::Link {
                    target: target.next().unwrap().as_str().into(),
                    subtarget: match target.next() {
                        Some(subtarget) if subtarget.as_rule() == Rule::heading_link => {
                            Subtarget::Heading(subtarget.as_str())
                        }
                        Some(subtarget) if subtarget.as_rule() == Rule::reference_link => {
                            Subtarget::Reference(subtarget.as_str())
                        }
                        Some(rule) => panic!("Invalid rule inside link: {rule}"),
                        _ => Subtarget::None
                    },
                    display: inner.next().map(|pair| pair.as_str()),
                    target_str
                }
            }
            Rule::tag => I::Tag,
            Rule::reference => I::Reference,
            Rule::comment => I::Comment,
            Rule::external_link => I::ExternalLink {
                display: inner
                    .next_if(|pair| pair.as_rule() == Rule::external_link_display)
                    .map(|pair| pair.as_str()),
                target: inner.next().unwrap().as_str().parse().unwrap()
            },
            Rule::external_embed => {
                let display = inner
                    .next_if(|pair| pair.as_rule() == Rule::external_link_display)
                    .map(|pair| pair.as_str());
                let (target, task) =
                    file_store.open(inner.next().unwrap().as_str().parse().unwrap());
                tasks.push(task);
                I::ExternalEmbed { target, display }
            }
            Rule::soft_break => I::SoftBreak,
            rule => panic!("Invalid rule inside line: {rule:?}")
        };
        ParagraphItem { span, kind }
    }
}

#[derive(Debug)]
pub struct ParagraphItem<'a> {
    pub span: Span<'a>,
    pub kind: ParagraphItemKind<'a>
}

#[derive(Debug)]
pub enum ParagraphItemKind<'a> {
    ModifierSpan(Modifiers, Vec<ParagraphItem<'a>>),
    Text,
    InlineCodeBlock {
        inner: &'a str
    },
    InlineMathBlock {
        inner: &'a str
    },
    SoftBreak,
    Link {
        target_str: &'a str,
        target: PathBuf,
        subtarget: Subtarget<'a>,
        display: Option<&'a str>
    },
    ExternalLink {
        target: FileLocator,
        display: Option<&'a str>
    },

    ExternalEmbed {
        display: Option<&'a str>,
        target: Rc<FileData>
    },

    Embed {
        target_str: &'a str,
        target: Rc<FileData>,
        subtarget: Subtarget<'a>,
        display: Option<&'a str>
    },

    EscapedChar,
    Tag,
    Reference,
    Comment
}

// TODO: Bitflags make me do long if else chains instead of match arms, maybe a struct of bools would be better
bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct Modifiers: u16 {
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
        const HIGHLIGHT = 1 << 2;
        const STRIKETHROUGH = 1 << 3;
        const CODE = 1 << 4;
        const TAG = 1 << 5;
        const REFERENCE = 1 << 7;
        const UNSUPPORTED = 1 << 8;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Subtarget<'a> {
    None,
    Heading(&'a str),
    Reference(&'a str)
}

#[derive(Debug)]
pub enum ListItemType {
    Bullet,
    Numbered(u16),
    Task(bool)
}
