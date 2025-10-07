// TODO: Switch to yoke
use std::{
    alloc::{self, Layout},
    mem::ManuallyDrop,
    ptr::NonNull
};

use pokisona_markdown::Markdown;

#[derive(Debug)]
pub struct MarkdownStore {
    markdown: ManuallyDrop<Markdown<'static>>,
    source: NonNull<str>
}

impl MarkdownStore {
    pub fn new(source: String) -> Self {
        let source = source.leak();
        let source_ptr = NonNull::from_mut(source);
        let markdown = ManuallyDrop::new(Markdown::parse(source));
        Self {
            markdown,
            source: source_ptr
        }
    }

    pub fn _get<'a>(&'a self) -> &'a Markdown<'a> {
        &self.markdown
    }

    pub fn source(&self) -> &str {
        unsafe { self.source.as_ref() }
    }
}

impl Drop for MarkdownStore {
    fn drop(&mut self) {
        //SAFETY: The markdown returned from get lives as long as the store so nothing is referencing the source
        unsafe {
            ManuallyDrop::drop(&mut self.markdown);
            let ptr = self.source.as_ptr() as *mut u8;
            let layout = Layout::array::<u8>(self.source.as_ref().len()).unwrap();
            alloc::dealloc(ptr, layout)
        }
    }
}

unsafe impl Sync for MarkdownStore {}
unsafe impl Send for MarkdownStore {}
