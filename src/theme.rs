use catppuccin::{ColorName, FlavorColors};
use iced::{
    Color, Shadow, Vector, border,
    theme::{Mode, palette},
    widget::{
        self, button, container, rule,
        scrollable::{self, AutoScroll, Rail, Scroller},
        text_editor, text_input,
    },
};

pub const CATPPUCCIN: FlavorColors = catppuccin::PALETTE.frappe.colors;
const BORDER_RADIUS: f32 = 4.0;

// TODO: add configurble accents and flavors
pub struct CatppuccinFrappe;
impl iced::theme::Base for CatppuccinFrappe {
    fn default(_preference: iced::theme::Mode) -> Self {
        Self
    }

    fn mode(&self) -> Mode {
        Mode::Dark
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: (CATPPUCCIN.mantle).into(),
            text_color: (CATPPUCCIN.text).into(),
        }
    }

    fn name(&self) -> &'static str {
        "Catppuccin frappe"
    }

    fn seed(&self) -> Option<palette::Seed> {
        None
    }
}

pub enum ContainerClass {
    None,
    Surface0,
    Surface1,
    Base,
    Tint,
    Tag { color: ColorName },
    BorderedBox { highlighted: bool },
}

impl container::Catalog for CatppuccinFrappe {
    type Class<'a> = ContainerClass;

    fn default<'a>() -> Self::Class<'a> {
        ContainerClass::None
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        match class {
            ContainerClass::Surface0 => container::Style::default().background(CATPPUCCIN.surface0),
            ContainerClass::Surface1 => container::Style::default().background(CATPPUCCIN.surface1),
            ContainerClass::Base => container::Style::default().background(CATPPUCCIN.base),
            ContainerClass::Tint => {
                return container::Style {
                    background: Some(Color::BLACK.scale_alpha(0.8).into()),
                    ..Default::default()
                };
            }
            ContainerClass::None => return container::Style::default(),
            ContainerClass::BorderedBox { highlighted } => {
                return container::Style::default()
                    .border(
                        border::rounded(BORDER_RADIUS)
                            .width(2.0)
                            .color(if *highlighted {
                                CATPPUCCIN.blue
                            } else {
                                CATPPUCCIN.overlay0
                            }),
                    )
                    .shadow(if *highlighted {
                        Shadow {
                            color: Color::from(CATPPUCCIN.blue).scale_alpha(0.2),
                            blur_radius: 10.0,
                            ..Default::default()
                        }
                    } else {
                        Shadow::default()
                    })
                    .background(CATPPUCCIN.surface1);
            }
            ContainerClass::Tag { color } => container::Style::default()
                .background(CATPPUCCIN[*color])
                .color(CATPPUCCIN.crust),
        }
        .border(border::rounded(BORDER_RADIUS))
    }
}

pub enum TextClass {
    Inherit,
    Overlay0,
    Danger,
}

impl widget::text::Catalog for CatppuccinFrappe {
    type Class<'a> = TextClass;

    fn default<'a>() -> Self::Class<'a> {
        TextClass::Inherit
    }

    fn style(&self, class: &Self::Class<'_>) -> widget::text::Style {
        let color = match class {
            TextClass::Inherit => return widget::text::Style::default(),
            TextClass::Overlay0 => CATPPUCCIN.overlay0,
            TextClass::Danger => CATPPUCCIN.red,
        }
        .into();
        widget::text::Style {
            color: Some(color),
            selection: None,
        }
    }

    fn selection(&self) -> Color {
        Color::from(CATPPUCCIN.blue).scale_alpha(0.5)
    }
}

pub enum ButtonClass {
    Primary,
    Secondary,
    Tag { highlight: bool, color: ColorName },
}

impl button::Catalog for CatppuccinFrappe {
    type Class<'a> = ButtonClass;

    fn default<'a>() -> Self::Class<'a> {
        ButtonClass::Primary
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        let (background, text_color) = match class {
            ButtonClass::Primary => (CATPPUCCIN.blue.into(), CATPPUCCIN.crust.into()),
            ButtonClass::Tag {
                highlight: true,
                color,
            } => (CATPPUCCIN[*color].into(), CATPPUCCIN.crust.into()),
            ButtonClass::Tag {
                highlight: false,
                color,
            } => (CATPPUCCIN.overlay0.into(), CATPPUCCIN[*color].into()),
            ButtonClass::Secondary => (CATPPUCCIN.crust.into(), CATPPUCCIN.text.into()),
        };

        let background = match status {
            button::Status::Active => background,
            button::Status::Hovered | button::Status::Pressed => palette::deviate(background, 0.1),
            button::Status::Disabled => background.scale_alpha(0.5),
        };

        button::Style {
            background: Some(background.into()),
            text_color,
            border: border::rounded(BORDER_RADIUS),
            ..Default::default()
        }
    }
}

impl scrollable::Catalog for CatppuccinFrappe {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        let scrollbar = Rail {
            background: Some((CATPPUCCIN.surface0).into()),
            border: border::rounded(BORDER_RADIUS),
            scroller: Scroller {
                background: (CATPPUCCIN.overlay2).into(),
                border: border::rounded(BORDER_RADIUS),
            },
        };

        let auto_scroll = AutoScroll {
            background: Color::from(CATPPUCCIN.base).scale_alpha(0.9).into(),
            border: border::rounded(u32::MAX)
                .width(1)
                .color(Color::from(CATPPUCCIN.text).scale_alpha(0.8)),
            shadow: Shadow {
                color: Color::BLACK.scale_alpha(0.7),
                offset: Vector::ZERO,
                blur_radius: 2.0,
            },
            icon: Color::from(CATPPUCCIN.text).scale_alpha(0.8),
        };

        match status {
            scrollable::Status::Active { .. } => scrollable::Style {
                container: container::Style::default(),
                vertical_rail: scrollbar,
                horizontal_rail: scrollbar,
                gap: None,
                auto_scroll,
            },
            scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
                ..
            } => {
                let hovered_scrollbar = Rail {
                    scroller: Scroller {
                        background: (CATPPUCCIN.overlay1).into(),
                        ..scrollbar.scroller
                    },
                    ..scrollbar
                };

                scrollable::Style {
                    container: container::Style::default(),
                    vertical_rail: if is_vertical_scrollbar_hovered {
                        hovered_scrollbar
                    } else {
                        scrollbar
                    },
                    horizontal_rail: if is_horizontal_scrollbar_hovered {
                        hovered_scrollbar
                    } else {
                        scrollbar
                    },
                    gap: None,
                    auto_scroll,
                }
            }
            scrollable::Status::Dragged {
                is_horizontal_scrollbar_dragged,
                is_vertical_scrollbar_dragged,
                ..
            } => {
                let dragged_scrollbar = Rail {
                    scroller: Scroller {
                        background: (CATPPUCCIN.base).into(),
                        ..scrollbar.scroller
                    },
                    ..scrollbar
                };

                scrollable::Style {
                    container: container::Style::default(),
                    vertical_rail: if is_vertical_scrollbar_dragged {
                        dragged_scrollbar
                    } else {
                        scrollbar
                    },
                    horizontal_rail: if is_horizontal_scrollbar_dragged {
                        dragged_scrollbar
                    } else {
                        scrollbar
                    },
                    gap: None,
                    auto_scroll,
                }
            }
        }
    }
}

impl text_editor::Catalog for CatppuccinFrappe {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _class: &Self::Class<'_>, _status: text_editor::Status) -> text_editor::Style {
        text_editor::Style {
            background: (CATPPUCCIN.surface0).into(),
            border: border::rounded(5.0),
            placeholder: (CATPPUCCIN.subtext0).into(),
            value: (CATPPUCCIN.text).into(),
            selection: Color::from(CATPPUCCIN.blue).scale_alpha(0.2),
        }
    }
}

impl rule::Catalog for CatppuccinFrappe {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _class: &Self::Class<'_>) -> rule::Style {
        rule::Style {
            color: CATPPUCCIN.overlay0.into(),
            radius: BORDER_RADIUS.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }
    }
}

impl text_input::Catalog for CatppuccinFrappe {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _class: &Self::Class<'_>, _status: text_input::Status) -> text_input::Style {
        text_input::Style {
            background: CATPPUCCIN.base.into(),
            border: border::rounded(BORDER_RADIUS)
                .width(1.0)
                .color(CATPPUCCIN.blue),
            placeholder: CATPPUCCIN.subtext0.into(),
            value: CATPPUCCIN.text.into(),
            selection: Color::from(CATPPUCCIN.blue).scale_alpha(0.2),
        }
    }
}
