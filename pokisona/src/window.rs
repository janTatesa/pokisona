use std::{
    mem,
    ops::{Index, IndexMut},
    sync::Arc
};

use crate::{
    column,
    file_store::FileData,
    markdown_view::render_markdown,
    row,
    widget::{ContainerKind, Spacing, Widget}
};

#[derive(Clone)]
pub struct WindowManager {
    root_node: WindowLayoutNode,
    windows_len: usize,
    current_window: usize
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            root_node: WindowLayoutNode::Window(Window::default()),
            windows_len: 1,
            current_window: 0
        }
    }
}

impl WindowManager {
    pub fn next_window(&mut self) {
        self.current_window = (self.current_window + 1) % self.windows_len;
    }

    pub fn previous_window(&mut self) {
        self.current_window = self
            .current_window
            .checked_sub(1)
            .unwrap_or(self.windows_len - 1);
    }

    pub fn add_window(&mut self, new_window: Window) {
        self.windows_len += 1;
        self.root_node.split_at(self.current_window, new_window);
        self.next_window();
    }

    #[must_use = "If none is returned the application should quit"]
    pub fn remove_window(&mut self) -> Option<Window> {
        let window = self.root_node.remove_split_at(self.current_window)?;
        self.windows_len -= 1;
        self.previous_window();
        Some(window)
    }

    pub fn render(&self) -> Widget<'_> {
        self.root_node.render(self.current_window)
    }

    pub fn current_window(&self) -> &Window {
        &self.root_node[self.current_window]
    }

    pub fn current_window_mut(&mut self) -> &mut Window {
        &mut self.root_node[self.current_window]
    }
}

#[derive(Clone)]
enum WindowLayoutNode {
    Window(Window),
    Split(Box<(WindowLayoutNode, WindowLayoutNode)>)
}

impl Default for WindowLayoutNode {
    fn default() -> Self {
        Self::Window(Window::default())
    }
}

enum Direction {
    Horizontal,
    Vertical
}

impl WindowLayoutNode {
    fn render(&self, focused_idx: usize) -> Widget<'_> {
        match self {
            WindowLayoutNode::Window(window) => {
                Widget::container(window.render(), ContainerKind::Padded)
            }
            WindowLayoutNode::Split(_) => Widget::container(
                self.render_nested(Some(focused_idx), Direction::Vertical),
                ContainerKind::Mantle
            )
        }
    }

    fn render_nested(&self, focused_idx: Option<usize>, direction: Direction) -> Widget<'_> {
        match (self, focused_idx, direction) {
            (WindowLayoutNode::Window(window), Some(0), _) => {
                Widget::container(window.render(), ContainerKind::BorderedBoxFocused)
            }
            (WindowLayoutNode::Window(window), ..) => {
                Widget::container(window.render(), ContainerKind::BorderedBox)
            }
            (WindowLayoutNode::Split(nodes), _, Direction::Horizontal) => column![
                Spacing::Normal,
                nodes.0.render_nested(focused_idx, Direction::Vertical),
                nodes.1.render_nested(
                    focused_idx.and_then(|idx| idx.checked_sub(nodes.0.len())),
                    Direction::Vertical
                )
            ],
            (WindowLayoutNode::Split(nodes), _, Direction::Vertical) => row![
                Spacing::Normal,
                nodes.0.render_nested(focused_idx, Direction::Horizontal),
                nodes.1.render_nested(
                    focused_idx.and_then(|idx| idx.checked_sub(nodes.0.len())),
                    Direction::Horizontal
                )
            ]
        }
    }

    fn split_at(&mut self, index: usize, new_window: Window) {
        match self {
            WindowLayoutNode::Window(window) => {
                let old_window = WindowLayoutNode::Window(mem::take(window));
                let new_window = WindowLayoutNode::Window(new_window);
                *self = WindowLayoutNode::Split(Box::new((old_window, new_window)))
            }
            WindowLayoutNode::Split(nodes) => {
                let first_node_len = nodes.0.len();
                if first_node_len > index {
                    return nodes.0.split_at(index, new_window);
                }

                nodes.1.split_at(index - first_node_len, new_window);
            }
        }
    }

    fn remove_split_at(&mut self, index: usize) -> Option<Window> {
        match self {
            WindowLayoutNode::Window(_) => None,
            WindowLayoutNode::Split(nodes) => {
                if let WindowLayoutNode::Window(window) = &mut nodes.0
                    && index == 0
                {
                    let removed = Some(mem::take(window));
                    *self = mem::take(&mut nodes.1);
                    return removed;
                }

                let first_node_len = nodes.0.len();

                if first_node_len > index {
                    return nodes.0.remove_split_at(index);
                }

                if let WindowLayoutNode::Window(window) = &mut nodes.1 {
                    let removed = Some(mem::take(window));
                    *self = mem::take(&mut nodes.0);
                    return removed;
                }

                nodes.1.remove_split_at(index)
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            WindowLayoutNode::Window(_) => 1,
            WindowLayoutNode::Split(nodes) => nodes.0.len() + nodes.1.len()
        }
    }
}

impl Index<usize> for WindowLayoutNode {
    type Output = Window;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Self::Window(window) if index == 0 => window,
            Self::Window(_) => panic!("Index {index} out of bounds in window layout"),
            Self::Split(nodes) => {
                let first_node_len = nodes.0.len();
                if first_node_len > index {
                    return &nodes.0[index];
                }

                &nodes.1[index - first_node_len]
            }
        }
    }
}

impl IndexMut<usize> for WindowLayoutNode {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            Self::Window(window) if index == 0 => window,
            Self::Window(_) => {
                panic!("Index {index} out of bounds in window layout")
            }
            Self::Split(nodes) => {
                let first_node_len = nodes.0.len();
                if first_node_len > index {
                    return &mut nodes.0[index];
                }

                &mut nodes.1[index - first_node_len]
            }
        }
    }
}

#[derive(Default, Clone)]
#[allow(dead_code)]
pub enum Window {
    #[default]
    Empty,
    Markdown(Arc<FileData>)
}

impl Window {
    fn render(&self) -> Widget<'_> {
        match self {
            Window::Empty => Widget::Space,
            Window::Markdown(file_data) => file_data
                .content()
                .map(|content| render_markdown(content.markdown()))
                .into()
        }
    }
}
