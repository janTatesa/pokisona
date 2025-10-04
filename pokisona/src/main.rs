use pokisona_markdown::Root;
fn main() {
    dbg!(Root::parse(include_str!("../test.md")));
}
