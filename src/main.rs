#![allow(dead_code)]

use bitflags::bitflags;

fn main() {
    println!("Hello, world!");
}

struct Root {
    yaml: String,
    content: Content,
}

struct Content {
    blocks: Vec<Block>,
    headings: Vec<Content>,
}

enum Block {
    Paragraph(Paragraph),
    CodeBlock(CodeBlock),
    ListItem(ListItem),
    Table(Table),
    Quote(Vec<RichTextSpan>),
    Callout(Callout),
    Embed(Embed),
    MathBlock(MathBlock),
}

struct Paragraph {
    text: Vec<RichTextSpan>,
    reference: Option<String>,
}

enum RichTextSpan {
    TextWithModifiers(String, Modifiers),
    InlineCodeBlock(String),
    InlineMathBlock(String),
    Link(Link),
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

struct Link {
    destination: String,
    reference: LinkReference,
    display: String,
}

enum LinkReference {
    None,
    Heading(String),
    Block(String),
}

struct CodeBlock {
    language: String,
    content: String,
    reference: Option<String>,
}

struct ListItem {
    content: Vec<RichTextSpan>,
    r#type: ListItemType,
    subitems: Vec<ListItem>,
    reference: Option<String>,
}

enum ListItemType {
    Bullet,
    Numbered(u16),
    Task(bool),
}

struct Table {
    columns: Vec<TableColumn>,
    reference: Option<String>,
}

struct TableColumn {
    name: RichTextSpan,
    alignment: ColumnAlignment,
    rows: Vec<RichTextSpan>,
}

enum ColumnAlignment {
    Left,
    Right,
    Center,
}

struct Callout {
    r#type: String,
    title: String,
    content: Content,
}

struct Embed {
    link: Link,
    reference: Option<String>,
}

struct MathBlock {
    content: String,
    reference: Option<String>,
}
