mod iced_helpers;
mod markdown;
mod theme;

use iced::{
    Alignment::{self},
    Border, Length, Padding, Shadow, Vector,
    border::{self, Radius},
    widget::{
        self, PaneGrid, column,
        pane_grid::{Content, Controls, DragEvent, TitleBar},
        row, stack
    }
};
use iced_helpers::{BORDER_WIDTH, container, text};
pub use theme::Theme;

use crate::{
    Element, ElementId, HoveredLink, Message, Pokisona,
    command::Command,
    view::iced_helpers::{BORDER_RADIUS, PADDING, SPACING, button, button_enabled_if}
};

impl Pokisona {
    pub(super) fn view(&self) -> Element<'_> {
        let theme = self.config.theme;
        let vault_name = container(self.vault_name.as_str())
            .align_x(Alignment::End)
            .width(Length::Fill);

        let bar_content: Element<'_> = match (
            self.command_history.currently_selected(),
            self.typed_command.as_deref(),
            &self.error
        ) {
            (Some(command), ..) | (_, Some(command), _) => row![
                ":",
                widget::sensor(
                    widget::text_input("Enter command", command)
                        .on_input(|input| Command::CommandLineSet(input).into())
                        .on_submit(Command::CommandLineSubmit.into())
                        .id(ElementId::CommandInput)
                        .padding(0)
                )
                .on_show(|_| Message::Focus(ElementId::CommandInput)),
                vault_name
            ]
            .spacing(0)
            .into(),
            (_, _, Some(error)) => row![text(error, theme.danger), vault_name].into(),
            _ => vault_name.into()
        };

        let bar = container(bar_content)
            .background(theme.crust)
            .width(Length::Fill);
        let hovered_link = self.hovered_link.as_ref().and_then(|link| {
            let border = border::rounded(BORDER_RADIUS)
                .color(theme.overlay0)
                .width(BORDER_WIDTH);

            let hovered_link = match link {
                HoveredLink::Internal(file_data) => {
                    let bar = container(file_data.path().as_str())
                        .align_x(Alignment::Center)
                        .width(Length::Fill)
                        .border(border::rounded(Radius::default().bottom(BORDER_RADIUS)))
                        .custom_padding(Padding::default().left(PADDING).right(PADDING))
                        .background(theme.crust);
                    let content = container(file_data.content()?.view(theme)).padded();
                    let content = column![content, bar].padding(BORDER_WIDTH);

                    container(content)
                }
                HoveredLink::Error(url) => container(text(url, theme.danger)).padded(),
                HoveredLink::External(url) => container(text(url, theme.link_external)).padded()
            }
            .background(theme.base)
            .border(border)
            .shadowed();
            const CURSOR_HOVER_OFFSET: f32 = 8.;
            let hovered_link_pos =
                self.mouse_pos + Vector::new(CURSOR_HOVER_OFFSET, CURSOR_HOVER_OFFSET);
            let padding = Padding::default()
                .top(hovered_link_pos.y)
                .left(hovered_link_pos.x);
            let hovered_link = container(hovered_link).custom_padding(padding);
            Some(hovered_link)
        });

        let windows = PaneGrid::new(&self.panes, |pane, state, _maximised| {
            let title = state
                .current_file()
                .map(|file| {
                    let path = file.path().as_str();
                    path.strip_suffix(".md").unwrap_or(path)
                })
                .unwrap_or("[scratch]");
            let (text_color, background, button_disabled_color) = if pane == self.focus {
                (theme.text, theme.mantle, theme.overlay0)
            } else {
                (theme.subtext0, theme.crust, theme.overlay1)
            };

            let controls = || {
                row![
                    button_enabled_if(
                        Command::FileHistoryBackward,
                        text_color,
                        button_disabled_color,
                        self.panes.get(self.focus).unwrap().can_go_backward()
                    ),
                    button_enabled_if(
                        Command::FileHistoryForward,
                        text_color,
                        button_disabled_color,
                        self.panes.get(self.focus).unwrap().can_go_forward()
                    ),
                    button(
                        Command::VSplit {
                            path: None,
                            pane: Some(pane)
                        },
                        text_color
                    ),
                    button(
                        Command::HSplit {
                            path: None,
                            pane: Some(pane)
                        },
                        text_color
                    ),
                    button(Command::Quit(Some(pane)), text_color)
                ]
                .spacing(SPACING)
            };

            // HACK: this is the only way to currently have both centered title while controls being displayed. The compact controls are always displayed.
            let controls = Controls::dynamic("This shouldn't be displayed", controls());

            let title = row![
                container(text("Normal", theme.crust)).background(theme.accent),
                widget::container(text(title, text_color)).center_x(Length::Fill)
            ]
            .spacing(SPACING);
            let title_bar = TitleBar::new(title)
                .always_show_controls()
                .controls(controls)
                .style(move |_| widget::container::Style {
                    text_color: Some(text_color),
                    background: Some(background.into()),
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: false
                });
            let body = state.current_file().map(|file| {
                let body = container(file.content()?.view(self.theme()))
                    .padded()
                    .stretched();
                Some(body)
            });
            /// Since the border is displayed on two panes next to each other, the border width has to be halved
            const CONTENT_BORDER_WIDTH: f32 = BORDER_WIDTH / 2.;
            Content::new(body)
                .title_bar(title_bar)
                .style(move |_| widget::container::Style {
                    text_color: None,
                    background: None,
                    border: border::width(CONTENT_BORDER_WIDTH).color(background),
                    shadow: Shadow::default(),
                    snap: false
                })
        })
        .on_click(|pane| Command::FocusPane(pane).into())
        .on_drag(|event| {
            match event {
                DragEvent::Dropped { pane, target } => Command::DropPane { pane, target },
                _ => Command::Noop
            }
            .into()
        })
        .on_resize(BORDER_WIDTH, |event| Command::ResizePane(event).into());

        let ui = column![windows, bar];
        stack![ui, hovered_link].into()
    }
}
