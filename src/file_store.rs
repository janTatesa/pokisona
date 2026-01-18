// TODO: switch to smth more robust
use std::{
    cell::OnceCell,
    rc::{Rc, Weak}
};

use dashmap::DashMap;
use iced::Task;
use smol::fs;

use crate::{
    Message, Path, PathBuf,
    command::Command,
    markdown::{Markdown, store::MarkdownStore}
};

// TODO: maybe use RefCell hashmap
#[derive(Default, Debug)]
pub struct FileStore(DashMap<PathBuf, Weak<FileData>>);

#[derive(Debug)]
pub struct FileData {
    path: PathBuf,
    file_store: &'static FileStore,
    content: OnceCell<MarkdownStore>
}

impl FileData {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> Option<&Markdown<'_>> {
        self.content.get().map(MarkdownStore::inner)
    }
}

impl Drop for FileData {
    fn drop(&mut self) {
        self.file_store.0.remove(self.path());
    }
}

impl FileStore {
    /// Creates a reference to a file and a task which produces [`Message::FileOpened`]
    pub fn open_file(&'static self, path: PathBuf) -> (Rc<FileData>, Task<Message>) {
        if let Some(weak) = self.0.get(&path)
            && let Some(data) = weak.upgrade()
        {
            return (data, Task::none());
        }

        let data = Rc::new(FileData {
            path: path.clone(),
            content: OnceCell::new(),
            file_store: self
        });

        self.0.insert(path.clone(), Rc::downgrade(&data));
        let future = Task::future(async {
            let content = fs::read_to_string(&path)
                .await
                .map_err(|error| error.to_string());
            content
                .map(|content| Message::FileOpened { path, content })
                .unwrap_or_else(|err| Command::Error(err).into())
        });
        (data, future)
    }

    pub fn insert(&self, path: &Path, content: MarkdownStore) {
        if let Some(weak) = self.0.get(path)
            && let Some(data) = weak.upgrade()
            && data.content.set(content).is_err()
        {
            panic!("Attemted to read a single file twice")
        }
    }
}
