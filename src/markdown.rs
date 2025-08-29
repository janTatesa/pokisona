#![allow(dead_code)]

mod parsing;

use std::ops::Range;

use bitflags::bitflags;
use serde_yml::Mapping;

pub type Span = Range<usize>;
#[derive(Debug)]
pub struct Root<'a> {
    pub yaml: Option<Yaml>,
    pub content: Vec<Block<'a>>,
}

#[derive(Debug)]
pub struct Yaml {
    pub yaml: Result<Mapping, serde_yml::Error>,
    pub inner_span: Span,
    pub span: Span,
}

#[derive(Debug)]
pub struct Heading<'a> {
    pub name: &'a str,
    pub content: Vec<Block<'a>>,
    pub title_span: Span,
    pub span: Span,
}

#[derive(Debug)]
pub enum Block<'a> {
    Paragraph(Paragraph<'a>),
    CodeBlock(CodeBlock<'a>),
    ListItem(ListItem<'a>),
    Table(Table<'a>),
    Quote(Quote<'a>),
    Callout(Callout<'a>),
    Embed(Embed<'a>),
    MathBlock(MathBlock<'a>),
    FootnoteDeclaration(FootnoteDeclaration<'a>),
    Heading(Heading<'a>),
    Ruler(Span),
}

#[derive(Debug)]
pub struct Paragraph<'a> {
    pub text: Vec<RichTextSpan<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub enum RichTextSpan<'a> {
    TextWithModifiers(TextWithModifiers<'a>),
    InlineCodeBlock(&'a str),
    InlineMathBlock(&'a str),
    InlineFootnote(&'a str),
    Footnote(&'a str),
    Link(Link<'a>),
    Tag(&'a str),
    Reference(&'a str),
}

#[derive(Debug)]
pub struct TextWithModifiers<'a> {
    pub text: &'a str,
    pub modifiers: Modifiers,
    pub span: Span,
}

bitflags! {
    #[derive(Debug)]
    pub struct Modifiers: u8 {
        const NONE = 0b00000000;
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const HIGHLIGHT = 0b00000100;
        const STRIKETHROUGH = 0b00001000;
    }
}

#[derive(Debug)]
pub struct Link<'a> {
    pub destination: &'a str,
    pub reference: LinkReference<'a>,
    pub display: &'a str,
    pub span: Span,
}

#[derive(Debug)]
pub enum LinkReference<'a> {
    None,
    Heading(&'a str),
    Block(&'a str),
}

#[derive(Debug)]
pub struct CodeBlock<'a> {
    pub language: &'a str,
    pub content: &'a str,
    pub span: Span,
}

#[derive(Debug)]
pub struct ListItem<'a> {
    pub content: Vec<RichTextSpan<'a>>,
    pub r#type: ListItemType,
    pub subitems: Vec<ListItem<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ListItemType {
    Bullet,
    Numbered(u16),
    Task(bool),
}

#[derive(Debug)]
pub struct Table<'a> {
    pub columns: Vec<TableColumn<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct TableColumn<'a> {
    pub name: RichTextSpan<'a>,
    pub alignment: ColumnAlignment,
    pub rows: Vec<RichTextSpan<'a>>,
}

#[derive(Debug)]
pub enum ColumnAlignment {
    Left,
    Right,
    Center,
}

#[derive(Debug)]
pub struct Quote<'a> {
    pub content: Vec<Block<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Callout<'a> {
    pub r#type: &'a str,
    pub title: &'a str,
    pub content: Vec<Block<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Embed<'a> {
    pub link: Link<'a>,
    pub span: Span,
}

#[derive(Debug)]
pub struct MathBlock<'a> {
    pub content: &'a str,
    pub span: Span,
}

#[derive(Debug)]
pub struct FootnoteDeclaration<'a> {
    pub footnote: &'a str,
    pub span: Span,
}
