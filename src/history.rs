use std::{fs, io, path::PathBuf, sync::LazyLock};

use serde::{Deserialize, Serialize};

use crate::View;

static PATH: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::data_dir().unwrap().join("pokisona/history.bin"));
#[derive(Serialize, Deserialize)]
pub struct History {
    views: Vec<View>,
    idx: usize,
    #[serde(skip)]
    modified: bool
}

impl Default for History {
    fn default() -> Self {
        Self {
            views: vec![View::Title],
            idx: 0,
            modified: false
        }
    }
}

impl History {
    pub fn new() -> io::Result<Self> {
        if PATH.exists() {
            Ok(postcard::from_bytes(&fs::read(&*PATH)?).unwrap())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save_if_needed(&mut self) -> io::Result<()> {
        if self.modified {
            fs::write(&*PATH, postcard::to_allocvec(self).unwrap())?;
            self.modified = false;
        }

        Ok(())
    }

    pub fn close_current(&mut self) {
        if self.views.len() != 1 {
            self.modified = true;
            self.views.remove(self.idx);
            self.idx -= 1;
        }
    }

    pub fn can_close_current(&self) -> bool {
        self.views.len() != 1
    }

    pub fn current_view(&self) -> &View {
        &self.views[self.idx]
    }

    pub fn current_view_mut(&mut self) -> &mut View {
        self.modified = true;
        &mut self.views[self.idx]
    }

    pub fn forward(&mut self) {
        if self.can_go_forward() {
            self.modified = true;
            self.idx += 1;
        }
    }

    pub fn backward(&mut self) {
        if self.can_go_backward() {
            self.modified = false;
            self.idx -= 1;
        }
    }

    pub fn insert(&mut self, view: View) {
        self.modified = true;
        self.idx += 1;
        if self.idx < self.views.len() {
            self.views.drain(self.idx..);
        }

        self.views.push(view);
    }

    pub fn can_go_forward(&self) -> bool {
        (self.idx + 1) < self.views.len()
    }

    pub fn can_go_backward(&self) -> bool {
        self.idx != 0
    }
}
