#![allow(dead_code)]

use bitflags::bitflags;
use serde_yml::Value;

pub struct Root<'a> {
    yaml: Option<Value>,
    content: Content<'a>,
}

pub struct Content<'a> {
    blocks: Vec<Block<'a>>,
    headings: Vec<Heading<'a>>,
}

pub struct Heading<'a> {
    name: &'a str,
    content: Content<'a>,
}

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
}

pub struct Paragraph<'a> {
    text: Vec<RichTextSpan<'a>>,
    reference: Option<&'a str>,
}

pub enum RichTextSpan<'a> {
    TextWithModifiers(&'a str, Modifiers),
    InlineCodeBlock(&'a str),
    InlineMathBlock(&'a str),
    InlineFootnote(&'a str),
    Footnote(&'a str),
    Link(Link<'a>),
    Tag(&'a str),
}

bitflags! {
    pub struct Modifiers: u8 {
        const NONE = 0b00000000;
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const HIGHLIGHT = 0b00000100;
        const STRIKETHROUGH = 0b00001000;
    }
}

pub struct Link<'a> {
    destination: &'a str,
    reference: LinkReference<'a>,
    display: &'a str,
}

pub enum LinkReference<'a> {
    None,
    Heading(&'a str),
    Block(&'a str),
}

pub struct CodeBlock<'a> {
    language: &'a str,
    content: &'a str,
    reference: Option<&'a str>,
}

pub struct ListItem<'a> {
    content: Vec<RichTextSpan<'a>>,
    r#type: ListItemType,
    subitems: Vec<ListItem<'a>>,
    reference: Option<&'a str>,
}

pub enum ListItemType {
    Bullet,
    Numbered(u16),
    Task(bool),
}

pub struct Table<'a> {
    columns: Vec<TableColumn<'a>>,
    reference: Option<&'a str>,
}

pub struct TableColumn<'a> {
    name: RichTextSpan<'a>,
    alignment: ColumnAlignment,
    rows: Vec<RichTextSpan<'a>>,
}

pub enum ColumnAlignment {
    Left,
    Right,
    Center,
}

pub struct Quote<'a> {
    content: Content<'a>,
    reference: Option<&'a str>,
}

pub struct Callout<'a> {
    r#type: &'a str,
    title: &'a str,
    content: Content<'a>,
}

pub struct Embed<'a> {
    link: Link<'a>,
    reference: Option<&'a str>,
}

pub struct MathBlock<'a> {
    content: &'a str,
    reference: Option<&'a str>,
}

pub struct FootnoteDeclaration<'a> {
    footnote: &'a str,
    reference: Option<&'a str>,
}
