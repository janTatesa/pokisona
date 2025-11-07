// TODO:c this contains a lot of spaghetti, maybe use a modified version of iced's panegrid
use std::{
    mem,
    ops::{Index, IndexMut},
    rc::Rc
};

use iced::{
    Alignment, Length,
    widget::{Column, Row, column}
};

use crate::{
    file_store::FileData,
    iced_helpers::{BorderType, Element, SPACING, container},
    theme::Theme
};
#[derive(Clone, Debug)]
pub struct WindowManager {
    root_node: WindowLayoutNode,
    windows_len: usize,
    current_window: usize,
    starting_direction: Direction
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            root_node: WindowLayoutNode::Window(Window::default()),
            windows_len: 1,
            current_window: 0,
            starting_direction: Direction::Vertical
        }
    }
}

impl WindowManager {
    pub fn transpose_windows(&mut self) {
        self.starting_direction = match self.starting_direction {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal
        }
    }

    pub fn next_window(&mut self) {
        self.current_window = (self.current_window + 1) % self.windows_len;
    }

    pub fn previous_window(&mut self) {
        self.current_window = self
            .current_window
            .checked_sub(1)
            .unwrap_or(self.windows_len - 1);
    }

    pub fn split(&mut self, new_window: Window) {
        match &mut self.root_node {
            WindowLayoutNode::Window(window) => {
                self.root_node = WindowLayoutNode::Split(
                    [mem::take(window), new_window]
                        .map(WindowLayoutNode::Window)
                        .to_vec()
                )
            }
            WindowLayoutNode::Split(nodes) => {
                WindowLayoutNode::split_at(
                    nodes,
                    self.current_window,
                    new_window,
                    None,
                    self.starting_direction
                );
            }
        }

        self.windows_len += 1;
        self.next_window();
    }

    pub fn split_at_direction(&mut self, new_window: Window, direction: Direction) {
        match &mut self.root_node {
            WindowLayoutNode::Window(window) => {
                self.root_node = WindowLayoutNode::Split(
                    [mem::take(window), new_window]
                        .map(WindowLayoutNode::Window)
                        .to_vec()
                );
                self.starting_direction = direction;
            }
            WindowLayoutNode::Split(nodes) => {
                WindowLayoutNode::split_at(
                    nodes,
                    self.current_window,
                    new_window,
                    Some(direction),
                    self.starting_direction
                );
            }
        }

        self.windows_len += 1;
        self.next_window();
    }

    #[must_use = "If none is returned the application should quit"]
    pub fn remove_window(&mut self) -> Option<Window> {
        let window = self.root_node.remove_window_at(self.current_window)?;
        self.windows_len -= 1;
        self.previous_window();
        Some(window)
    }

    pub fn render(&self, theme: Theme) -> Element<'_> {
        if let WindowLayoutNode::Window(window) = &self.root_node {
            return window.render(theme, false);
        }

        let content =
            self.root_node
                .render(Some(self.current_window), self.starting_direction, theme);
        container(content).stretched().into()
    }

    #[allow(dead_code)]
    pub fn current_window(&self) -> &Window {
        &self.root_node[self.current_window]
    }

    pub fn current_window_mut(&mut self) -> &mut Window {
        &mut self.root_node[self.current_window]
    }
}

#[derive(Clone, Debug)]
enum WindowLayoutNode {
    Window(Window),
    Split(Vec<WindowLayoutNode>)
}

impl Default for WindowLayoutNode {
    fn default() -> Self {
        Self::Window(Window::default())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical
}

impl WindowLayoutNode {
    fn render(
        &self,
        focused_idx: Option<usize>,
        direction: Direction,
        theme: Theme
    ) -> Element<'_> {
        match (self, focused_idx, direction) {
            (WindowLayoutNode::Window(window), Some(0), _) => window.render(theme, true),
            (WindowLayoutNode::Window(window), ..) => window.render(theme, false),
            (WindowLayoutNode::Split(nodes), _, Direction::Horizontal) => Column::from_iter(
                NodeIter {
                    i: nodes.iter(),
                    focused_idx
                }
                .map(|(node, focused_idx)| node.render(focused_idx, Direction::Vertical, theme))
            )
            .spacing(SPACING)
            .into(),
            (WindowLayoutNode::Split(nodes), _, Direction::Vertical) => Row::from_iter(
                NodeIter {
                    i: nodes.iter(),
                    focused_idx
                }
                .map(|(node, focused_idx)| node.render(focused_idx, Direction::Horizontal, theme))
            )
            .spacing(SPACING)
            .into()
        }
    }

    fn split_at(
        nodes: &mut Vec<Self>,
        idx: usize,
        new_window: Window,
        target_direction: Option<Direction>,
        current_direction: Direction
    ) {
        let i = nodes.iter_mut();
        let focused_idx = Some(idx);
        let (node, new_focused_idx) = NodeIter { i, focused_idx }
            .find_map(|(node, focused)| Some((node, focused?)))
            .expect("Index out of bounds");

        match node {
            WindowLayoutNode::Window(_)
                if target_direction.is_some_and(|target| target == current_direction) =>
            {
                nodes.insert(idx + 1, WindowLayoutNode::Window(new_window));
            }
            WindowLayoutNode::Window(window) => {
                *node = WindowLayoutNode::Split(
                    [mem::take(window), new_window]
                        .map(WindowLayoutNode::Window)
                        .to_vec()
                )
            }

            WindowLayoutNode::Split(window_layout_nodes) => Self::split_at(
                window_layout_nodes,
                new_focused_idx,
                new_window,
                target_direction,
                match current_direction {
                    Direction::Horizontal => Direction::Vertical,
                    Direction::Vertical => Direction::Horizontal
                }
            )
        }
    }

    #[must_use = "If none is returned the last window was closed and the app should quit"]
    fn remove_window_at(&mut self, focused_idx: usize) -> Option<Window> {
        match self {
            WindowLayoutNode::Window(_) => None,
            WindowLayoutNode::Split(window_layout_nodes) => {
                let (node, new_focused_idx) = NodeIter {
                    i: window_layout_nodes.iter_mut(),
                    focused_idx: Some(focused_idx)
                }
                .find_map(|(node, idx)| Some((node, idx?)))
                .expect("Index out of bounds");
                if let WindowLayoutNode::Window(window) = node {
                    let window = mem::take(window);
                    window_layout_nodes.remove(focused_idx);
                    if window_layout_nodes.len() == 1 {
                        *self = mem::take(window_layout_nodes.iter_mut().next().unwrap());
                    }

                    return Some(window);
                }

                node.remove_window_at(new_focused_idx)
            }
        }
    }
}

impl Index<usize> for WindowLayoutNode {
    type Output = Window;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            WindowLayoutNode::Window(window) => window,
            WindowLayoutNode::Split(window_layout_nodes) => {
                let (node, idx) = NodeIter {
                    i: window_layout_nodes.iter(),
                    focused_idx: Some(index)
                }
                .find_map(|(node, idx)| Some((node, idx?)))
                .expect("Index out of bounds");
                &node[idx]
            }
        }
    }
}

impl IndexMut<usize> for WindowLayoutNode {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            WindowLayoutNode::Window(window) => window,
            WindowLayoutNode::Split(window_layout_nodes) => {
                let (node, idx) = NodeIter {
                    i: window_layout_nodes.iter_mut(),
                    focused_idx: Some(index)
                }
                .find_map(|(node, idx)| Some((node, idx?)))
                .expect("Index out of bounds");
                &mut node[idx]
            }
        }
    }
}

trait WithLen {
    fn len(&self) -> usize;
}

impl WithLen for &WindowLayoutNode {
    fn len(&self) -> usize {
        match self {
            WindowLayoutNode::Window(_) => 1,
            WindowLayoutNode::Split(window_layout_nodes) => {
                window_layout_nodes.iter().map(|node| node.len()).sum()
            }
        }
    }
}

impl WithLen for &mut WindowLayoutNode {
    fn len(&self) -> usize {
        (&**self).len()
    }
}

struct NodeIter<I> {
    i: I,
    focused_idx: Option<usize>
}

impl<I> Iterator for NodeIter<I>
where
    I: Iterator,
    I::Item: WithLen
{
    type Item = (I::Item, Option<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.i.next()?;
        let len = node.len();
        let current_focused = match self.focused_idx {
            Some(focused) if focused < len => {
                self.focused_idx = None;
                Some(focused)
            }
            Some(focused) => {
                self.focused_idx = Some(focused - len);
                None
            }
            _ => None
        };

        Some((node, current_focused))
    }
}

#[derive(Default, Clone, Debug)]
#[allow(dead_code)]
pub enum Window {
    #[default]
    Empty,
    Markdown(Rc<FileData>)
}

impl Window {
    fn render(&self, theme: Theme, focused: bool) -> Element<'_> {
        let (main, bar) = match self {
            Window::Empty => (None, "[scratch]"),
            Window::Markdown(file_data) => (
                file_data
                    .content()
                    .map(|content| content.inner().render(theme)),
                file_data.path().as_str()
            )
        };

        let bar = container(bar)
            .align_x(Alignment::Center)
            .border(BorderType::TitleBarBottom)
            .color(theme.crust)
            .width(Length::Fill);

        let content = column![container(main).stretched().padded(), bar].clip(true);
        let border = if focused {
            BorderType::Focused
        } else {
            BorderType::Invisible
        };

        container(content)
            .border(border)
            .color(theme.base)
            .stretched()
            .into()
    }
}
