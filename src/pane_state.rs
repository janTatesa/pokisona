use std::rc::Rc;

use crate::file_store::FileData;

#[derive(Default, Debug)]
pub struct PaneState {
    history: Vec<Rc<FileData>>,
    index: Option<usize>
}

impl PaneState {
    pub fn new(file: Rc<FileData>) -> Self {
        Self {
            index: Some(0),
            history: vec![file]
        }
    }

    pub fn split(&self) -> Self {
        PaneState {
            history: self
                .index
                .map(|idx| self.history[idx].clone())
                .into_iter()
                .collect(),
            index: self.index.map(|_| 0)
        }
    }

    pub fn current_file(&self) -> Option<&FileData> {
        Some(&self.history[self.index?])
    }

    pub fn backward(&mut self) {
        self.index = self.index.map(|idx| idx.saturating_sub(1));
    }

    pub fn forward(&mut self) {
        let Some(idx) = self.index else {
            return;
        };

        self.index = Some((idx + 1).min(self.history.len() - 1));
    }

    pub fn can_go_backward(&self) -> bool {
        self.index.is_some_and(|idx| idx > 0)
    }

    pub fn can_go_forward(&self) -> bool {
        self.index.is_some_and(|idx| idx < self.history.len() - 1)
    }

    pub fn open(&mut self, file: Rc<FileData>) {
        let index = self.index.map(|idx| idx + 1).unwrap_or(0);
        if self.history.len() > index {
            self.history.drain(index..);
        }

        self.history.push(file);
        self.index = Some(index);
    }
}
