// TODO: switch to smth more robust
use std::{
    cell::OnceCell,
    rc::{Rc, Weak}
};

use dashmap::DashMap;

use crate::{Path, PathBuf, markdown::MarkdownStore};

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

    pub fn content(&self) -> Option<&MarkdownStore> {
        self.content.get()
    }
}

impl Drop for FileData {
    fn drop(&mut self) {
        self.file_store.0.remove(self.path());
    }
}

impl FileStore {
    // Returns the reference to file data and also if the reference was newly created
    pub fn get_ref(&'static self, path: PathBuf) -> (Rc<FileData>, bool) {
        if let Some(data) = self.0.get(&path).and_then(|weak| weak.upgrade()) {
            return (data, false);
        }

        let data = Rc::new(FileData {
            path: path.clone(),
            content: OnceCell::new(),
            file_store: self
        });

        self.0.insert(path, Rc::downgrade(&data));
        (data, true)
    }

    pub fn insert(&self, path: &Path, content: MarkdownStore) {
        if let Some(data) = self.0.get(path).and_then(|weak| weak.upgrade())
            && data.content.set(content).is_err()
        {
            panic!("Attemted to read a single file twice")
        }
    }
}
