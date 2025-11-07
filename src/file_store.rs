// TODO: switch to smth more robust
use std::sync::{Arc, LazyLock, OnceLock, Weak};

use dashmap::DashMap;

use crate::{Path, PathBuf, markdown::MarkdownStore};

// TODO: Maybe a leaked reference is better than a static
pub static FILE_STORE: LazyLock<FileStore> = LazyLock::new(FileStore::default);
#[derive(Default, Debug)]
pub struct FileStore(DashMap<PathBuf, Weak<FileData>>);

#[derive(Debug)]
pub struct FileData {
    path: PathBuf,
    content: OnceLock<MarkdownStore>
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
        if let Some(data) = self.0.get(path).and_then(|weak| weak.upgrade())
            && data.content.set(content).is_err()
        {
            panic!("Attemted to read a single file twice")
        }
    }
}
