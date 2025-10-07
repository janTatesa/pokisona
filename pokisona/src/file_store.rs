// TODO: switch to smth more robust
use std::{
    cell::{Cell, OnceCell},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, OnceLock, RwLock, Weak}
};

use dashmap::DashMap;
use hashbrown::HashMap;

use crate::markdown_store::MarkdownStore;

pub static FILE_STORE: LazyLock<FileStore> = LazyLock::new(FileStore::default);
#[derive(Default, Debug)]
pub struct FileStore(DashMap<PathBuf, Weak<FileData>>);

#[derive(Debug)]
pub struct FileData {
    path: PathBuf,
    content: OnceLock<MarkdownStore>
}

impl FileData {
    fn path(&self) -> &Path {
        &self.path
    }

    fn content(&self) -> Option<&MarkdownStore> {
        self.content.get()
    }
}

impl Drop for FileData {
    fn drop(&mut self) {
        FILE_STORE.0.remove(self.path());
    }
}

impl FileStore {
    // Returns the reference to file data and also if the reference was newly created
    pub fn get_ref(&self, path: PathBuf) -> (Arc<FileData>, bool) {
        if let Some(data) = self.0.get(&path).and_then(|weak| weak.upgrade()) {
            return (data, false);
        }
        let data = Arc::new(FileData {
            path: path.clone(),
            content: OnceLock::new()
        });

        self.0.insert(path, Arc::downgrade(&data));
        (data, true)
    }

    pub fn insert(&self, path: &Path, content: MarkdownStore) {
        if let Some(data) = self.0.get(path).and_then(|weak| weak.upgrade()) {
            data.content.set(content).unwrap();
        }
    }
}
