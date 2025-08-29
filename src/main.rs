mod markdown;

use pulldown_cmark::{Options, Parser};

use crate::markdown::Root;

fn main() {
    dbg!(Parser::new_ext(include_str!("./test.md"), Options::all()).collect::<Vec<_>>());
    dbg!(Root::parse(include_str!("./test.md")));
}
