use std::iter::Peekable;

use pulldown_cmark::{Event, OffsetIter, Options, Tag, TagEnd};

use crate::markdown::{Block, Paragraph, Root, Yaml};

type Parser<'a> = Peekable<OffsetIter<'a>>;
impl<'a> Root<'a> {
    pub fn parse(source: &'a str) -> Self {
        let parser = &mut pulldown_cmark::Parser::new_ext(source, Options::all())
            .into_offset_iter()
            .peekable();

        if let Some((Event::Start(Tag::MetadataBlock(kind)), span)) =
            parser.next_if(|(event, _)| matches!(event, Event::Start(Tag::MetadataBlock(_))))
        {
            let Some((Event::Text(yaml_text), inner_span)) = parser.next() else {
                panic!("Metadata tag should always be followed by text");
            };

            let ending_tag = parser.next();

            debug_assert_eq!(
                Some((Event::End(TagEnd::MetadataBlock(kind)), span.clone())),
                ending_tag
            );

            return Self {
                yaml: Some(Yaml {
                    yaml: serde_yml::from_str(&yaml_text),
                    inner_span,
                    span,
                }),
                content: BlockParser {
                    parser,
                    parent_heading_level: 0,
                }
                .collect(),
            };
        }

        Self {
            yaml: None,
            content: BlockParser {
                parser,
                parent_heading_level: 0,
            }
            .collect(),
        }
    }
}

struct BlockParser<'a, 'b> {
    parser: &'b mut Parser<'a>,
    parent_heading_level: u8,
}

impl<'a, 'b> Iterator for BlockParser<'a, 'b> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let (Event::Start(Tag::Heading { level, .. }), _) = self.parser.peek()?
            && (*level as u8) < self.parent_heading_level
        {
            return None;
        }

        let (event, span) = self.parser.next()?;
        Some(Block::Paragraph(Paragraph {
            text: Vec::new(),
            span: 0..0,
        }))
    }
}
