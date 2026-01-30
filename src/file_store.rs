use core::fmt;
// TODO: switch to smth more robust
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    convert::Infallible,
    fmt::{Display, Formatter},
    rc::{Rc, Weak},
    str::FromStr
};

use color_eyre::Result;
use iced::{Task, advanced::graphics::core::Bytes, widget::image::Handle};
use tokio::fs;

use crate::{Message, PathBuf, Url, command::Command, markdown::store::MarkdownStore};

#[derive(Default, Debug)]
pub struct FileStore(RefCell<HashMap<FileLocator, Weak<FileData>>>);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum FileLocator {
    Url(Url),
    Path(PathBuf)
}

impl FileLocator {
    pub fn as_str(&self) -> &str {
        match self {
            FileLocator::Url(url) => url.as_str(),
            FileLocator::Path(path) => path.as_str()
        }
    }
}

impl Display for FileLocator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<PathBuf> for FileLocator {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<Url> for FileLocator {
    fn from(url: Url) -> Self {
        Self::Url(url)
    }
}

impl FromStr for FileLocator {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Infallible> {
        Ok(Url::from_str(s)
            .ok()
            .filter(|url| !url.cannot_be_a_base())
            .map(Self::Url)
            .unwrap_or_else(|| Self::Path(PathBuf::from(s))))
    }
}

#[derive(Debug)]
pub struct FileData {
    locator: FileLocator,
    file_store: &'static FileStore,
    content: OnceCell<FileContent>
}

#[derive(Debug)]
pub enum FileContent {
    Markdown(MarkdownStore),
    Image(Handle),
    Unknown
}

impl FileData {
    pub fn locator(&self) -> &FileLocator {
        &self.locator
    }
    pub fn content(&self) -> Option<&FileContent> {
        self.content.get()
    }
}

impl Drop for FileData {
    fn drop(&mut self) {
        self.file_store.0.borrow_mut().remove(&self.locator);
    }
}

impl FileStore {
    /// Creates a reference to a file and a task which produces [`Message::FileOpened`]
    pub fn open(&'static self, mut locator: FileLocator) -> (Rc<FileData>, Task<Message>) {
        if let FileLocator::Path(path) = &mut locator
            && path.extension().is_none()
            && !path.exists()
        {
            path.set_extension("md");
        };

        if let Some(weak) = self.0.borrow().get(&locator)
            && let Some(data) = weak.upgrade()
        {
            return (data, Task::none());
        }

        let data = Rc::new(FileData {
            locator: locator.clone(),
            content: OnceCell::new(),
            file_store: self
        });

        self.0
            .borrow_mut()
            .insert(locator.clone(), Rc::downgrade(&data));
        let task = match locator {
            FileLocator::Url(url) => Task::future(async {
                async fn try_get(url: Url) -> Result<Message> {
                    let response = reqwest::get(url.clone()).await?;
                    Ok(Message::FileOpened {
                        locator: FileLocator::Url(url.clone()),
                        mime: response
                            .headers()
                            .get("Content-Type")
                            .and_then(|val| val.to_str().ok())
                            .map(ToString::to_string),
                        content: response.bytes().await?
                    })
                }

                try_get(url)
                    .await
                    .unwrap_or_else(|err| Command::Error(err.to_string()).into())
            }),
            FileLocator::Path(path) => Task::future(async {
                // TODO: for markdown files the content is converted to Bytes only to be converted back to Vec, maybe it could be read to Bytes directly?
                fs::read(&path)
                    .await
                    .map(|content| Message::FileOpened {
                        locator: FileLocator::Path(path),
                        content: Bytes::from(content),
                        mime: None
                    })
                    .unwrap_or_else(|err| Command::Error(err.to_string()).into())
            })
        };
        (data, task)
    }

    pub fn insert(
        &'static self,
        locator: &FileLocator,
        content: Bytes,
        mime: Option<&str>
    ) -> Task<Message> {
        let (content, task) = match (locator, mime) {
            (_, Some("text/markdown")) => {
                if let Ok(string) = String::from_utf8(content.to_vec()) {
                    let (store, task) = MarkdownStore::new(string, self);
                    (FileContent::Markdown(store), task)
                } else {
                    (FileContent::Unknown, Task::none())
                }
            }
            (FileLocator::Path(path), _) if path.extension() == Some("md") => {
                if let Ok(string) = String::from_utf8(content.to_vec()) {
                    let (store, task) = MarkdownStore::new(string, self);
                    (FileContent::Markdown(store), task)
                } else {
                    (FileContent::Unknown, Task::none())
                }
            }
            // TODO: i am too lazy to check for all possible image format extension so lets assume it's an image rn
            _ => (
                FileContent::Image(Handle::from_bytes(content)),
                Task::none()
            )
        };

        if let Some(weak) = self.0.borrow().get(locator)
            && let Some(data) = weak.upgrade()
            && data.content.set(content).is_err()
        {
            panic!("Attemted to read a single file twice")
        }

        task
    }
}
