use serde_yml::Value;

use crate::markdown::{ColumnAlignment, Link};

pub enum Token<'a> {
    Yaml(Value),
    HeadingStart(&'a str),
    Span(&'a str),
    Reference(&'a str),
    Asterisk,
    Underscore,
    TwoEquals,
    TwoTildes,
    InlineCodeBlock(&'a str),
    CodeBlock(CodeBlock<'a>),
    Bullet,
    TaskField(bool),
    NumberedListItemStart(u16),
    Pipe,
    TableColumnDeclaration(ColumnAlignment),
    GreaterThan,
    CalloutStart(&'a str),
    Link(Link<'a>),
    Tag(&'a str),
    InlineMathBlock(&'a str),
    MathBlock(&'a str),
}

pub struct CodeBlock<'a> {
    language: &'a str,
    content: &'a str,
}
