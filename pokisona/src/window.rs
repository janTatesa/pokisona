use std::{
    mem,
    ops::{Index, IndexMut},
    path::PathBuf
};

use iced::{
    Background, Border, Element,
    Length::Fill,
    widget::{column, container, row, scrollable, text}
};

use crate::{
    app::Message,
    color::{ACCENT, CRUST, MANTLE},
    markdown_store::MarkdownStore
};

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

    /// If none is returned the application should quit
    pub fn remove_window(&mut self) -> Option<Window> {
        let window = self.root_node.remove_split_at(self.current_window)?;
        self.windows_len -= 1;
        self.previous_window();
        Some(window)
    }

    pub fn render(&self) -> Element<'_, Message> {
        self.root_node.render(self.current_window)
    }

    pub fn set_current_window(&mut self, new_window: Window) {
        self.root_node[self.current_window] = new_window;
    }
}

#[derive(Debug)]
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
    fn render(&self, focused_idx: usize) -> Element<'_, Message> {
        match self {
            WindowLayoutNode::Window(window) => window.render(),
            WindowLayoutNode::Split(_) => self.render_nested(Some(focused_idx), Direction::Vertical)
        }
    }

    // TODO: Here's a lot of repeated code
    fn render_nested(
        &self,
        focused_idx: Option<usize>,
        direction: Direction
    ) -> Element<'_, Message> {
        match (self, focused_idx, direction) {
            (WindowLayoutNode::Window(window), Some(0), _) => container(window.render())
                .style(|_| {
                    container::Style::default()
                        .border(Border::default().rounded(5.0).width(2.5).color(ACCENT))
                        .background(Background::Color(MANTLE))
                })
                .width(Fill)
                .height(Fill)
                .into(),
            (WindowLayoutNode::Window(window), ..) => container(window.render())
                .style(|_| {
                    container::Style::default()
                        .border(Border::default().rounded(5.0))
                        .background(Background::Color(CRUST))
                })
                .width(Fill)
                .height(Fill)
                .into(),
            (WindowLayoutNode::Split(nodes), _, Direction::Horizontal) => container(
                column![
                    nodes.0.render_nested(focused_idx, Direction::Vertical),
                    nodes.1.render_nested(
                        focused_idx.and_then(|idx| idx.checked_sub(nodes.0.len())),
                        Direction::Vertical
                    )
                ]
                .spacing(5.0)
            )
            .width(Fill)
            .height(Fill)
            .into(),
            (WindowLayoutNode::Split(nodes), _, Direction::Vertical) => container(
                row![
                    nodes.0.render_nested(focused_idx, Direction::Horizontal),
                    nodes.1.render_nested(
                        focused_idx.and_then(|idx| idx.checked_sub(nodes.0.len())),
                        Direction::Horizontal
                    )
                ]
                .spacing(5.0)
            )
            .width(Fill)
            .height(Fill)
            .into()
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

#[derive(Debug, Default)]
#[allow(dead_code)]
pub enum Window {
    #[default]
    Empty,
    Markdown(PathBuf, MarkdownStore)
}

impl Window {
    fn render(&self) -> Element<'_, Message> {
        container(
            scrollable(text(format!("{self:#?}"))).style(|theme, status| {
                let mut style = scrollable::default(theme, status);
                style.vertical_rail.scroller.color = ACCENT;
                style
            })
        )
        .width(Fill)
        .height(Fill)
        .padding(5.0)
        .into()
    }
}
