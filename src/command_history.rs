use std::cmp::min;

#[derive(Default)]
pub struct CommandHistory {
    content: Vec<String>,
    selected: Option<usize>
}

impl CommandHistory {
    pub fn push(&mut self, command: String) {
        if self.content.last().is_none_or(|last| last != &command) {
            self.content.push(command.to_string());
        }
    }

    pub fn currently_selected(&self) -> Option<&str> {
        Some(&self.content[self.selected?])
    }

    pub fn select_up(&mut self) {
        if self.content.is_empty() {
            return;
        }

        let new = self
            .selected
            .unwrap_or(self.content.len())
            .saturating_sub(1);
        self.selected = Some(new)
    }

    pub fn select_down(&mut self) {
        self.selected = match (self.selected, self.content.len()) {
            (None, _) | (_, 0) => None,
            (Some(selected), len) if selected == len - 1 => None,
            (Some(selected), len) => Some(min(selected + 1, len - 1))
        }
    }

    pub fn deselect(&mut self) {
        self.selected = None;
    }
}
