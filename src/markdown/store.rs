// TODO: Switch to yoke
use std::{
    alloc::{self, Layout},
    mem::ManuallyDrop,
    ptr::NonNull
};

use iced::Task;

use crate::{Message, file_store::FileStore, markdown::Markdown};

#[derive(Debug)]
pub struct MarkdownStore {
    markdown: ManuallyDrop<Markdown<'static>>,
    source: NonNull<str>
}

impl MarkdownStore {
    pub fn new(input: String, file_store: &'static FileStore) -> (Self, Task<Message>) {
        let source = input.leak();
        let source_ptr = NonNull::from_mut(source);
        let (markdown, task) = Markdown::parse(source, file_store);
        let markdown_store = Self {
            markdown: ManuallyDrop::new(markdown),
            source: source_ptr
        };
        (markdown_store, task)
    }

    pub fn inner<'a>(&'a self) -> &'a Markdown<'a> {
        &self.markdown
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
