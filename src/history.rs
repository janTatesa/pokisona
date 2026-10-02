use std::cmp::min;

use crate::{View, markdown::Markdown};

pub struct History {
    views: Vec<View>,
    idx: usize
}

impl Default for History {
    fn default() -> Self {
        Self {
            views: vec![View::Title(Markdown::new(include_str!("../README.md")))],
            idx: 0
        }
    }
}

impl History {
    pub fn current_view(&self) -> &View {
        &self.views[self.idx]
    }

    pub fn current_view_mut(&mut self) -> &mut View {
        &mut self.views[self.idx]
    }

    pub fn forward(&mut self) {
        self.idx = min(self.idx + 1, self.views.len() - 1);
    }

    pub fn backward(&mut self) {
        self.idx = self.idx.saturating_sub(1);
    }

    pub fn insert(&mut self, view: View) {
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
