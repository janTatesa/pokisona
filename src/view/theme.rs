use catppuccin::{PALETTE, Rgb};
use iced::{
    Background, Border, Color, Shadow, border,
    theme::{Base, Mode, Palette, Style},
    widget::{
        button, checkbox, container, pane_grid,
        rule::{self, FillMode},
        scrollable::{self, AutoScroll, Rail},
        text, text_input
    }
};
use iced_selection::text as selection_text;
use serde::Deserialize;

use crate::view::iced_helpers::{self, ALPHA, BORDER_RADIUS, BORDER_WIDTH};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    flavor: Flavor,
    pub accent: Color,
    pub rainbow: [Color; 6],
    pub bold: Color,
    pub bold_italic: Color,
    pub italic: Color,
    pub link_internal: Color,
    pub link_external: Color,
    pub text: Color,
    pub subtext1: Color,
    pub subtext0: Color,
    pub overlay2: Color,
    pub overlay1: Color,
    pub overlay0: Color,
    pub surface2: Color,
    pub surface1: Color,
    pub surface0: Color,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        Ok(ThemeConfig::deserialize(deserializer)?.theme())
    }
}

impl From<Theme> for Option<iced::Theme> {
    fn from(_: Theme) -> Self {
        unimplemented!()
    }
}

impl Base for Theme {
    fn default(preference: Mode) -> Self {
        let flavor = match preference {
            Mode::None | Mode::Dark => Flavor::Mocha,
            Mode::Light => Flavor::Latte
        };

        ThemeConfig {
            flavor,
            accent: Accent::Mauve
        }
        .theme()
    }

    fn mode(&self) -> Mode {
        if self.flavor == Flavor::Latte {
            return Mode::Light;
        }

        Mode::Dark
    }

    fn base(&self) -> Style {
        Style {
            background_color: self.base,
            text_color: self.text
        }
    }

    fn palette(&self) -> Option<Palette> {
        let palette = match self.flavor {
            Flavor::Mocha => iced::Theme::CatppuccinMocha,
            Flavor::Macchiato => iced::Theme::CatppuccinMacchiato,
            Flavor::Frappe => iced::Theme::CatppuccinFrappe,
            Flavor::Latte => iced::Theme::CatppuccinLatte
        }
        .palette();
        Some(palette)
    }

    fn name(&self) -> &str {
        "catppuccin"
    }
}

macro_rules! catalog {
    ($widget:ident) => {
        catalog!($widget, |_| $widget::Style::default());
    };
    ($widget:ident, $style:expr) => {
        impl $widget::Catalog for Theme {
            type Class<'a> = $widget::StyleFn<'a, Self>;

            fn default<'a>() -> $widget::StyleFn<'a, Self> {
                Box::new($style)
            }

            fn style(&self, class: &$widget::StyleFn<'_, Self>) -> $widget::Style {
                class(self)
            }
        }
    };
    ($widget:ident, $status:path, $style:expr) => {
        impl $widget::Catalog for Theme {
            type Class<'a> = $widget::StyleFn<'a, Self>;

            fn default<'a>() -> Self::Class<'a> {
                Box::new($style)
            }

            fn style(&self, class: &Self::Class<'_>, status: $status) -> $widget::Style {
                class(self, status)
            }
        }
    };
}

catalog!(container);
catalog!(text);
catalog!(selection_text, |theme| selection_text::Style {
    color: None,
    selection: theme.accent.scale_alpha(ALPHA)
});

catalog!(text_input, text_input::Status, |theme, _| {
    text_input::Style {
        background: Background::Color(theme.crust),
        border: Border::default(),
        placeholder: theme.subtext1,
        value: theme.text,
        selection: theme.accent.scale_alpha(ALPHA),
        icon: Color::TRANSPARENT
    }
});

catalog!(rule, |theme| rule::Style {
    color: theme.accent,
    radius: BORDER_RADIUS.into(),
    fill_mode: FillMode::Full,
    snap: false
});

catalog!(checkbox, checkbox::Status, |theme, status| {
    use checkbox::Status as S;
    let border = if matches!(status, S::Hovered { .. }) {
        Border::default()
            .rounded(BORDER_RADIUS)
            .width(BORDER_WIDTH)
            .color(theme.accent)
    } else {
        Default::default()
    };

    let background = if let S::Active { is_checked: true }
    | S::Hovered { is_checked: true }
    | S::Disabled { is_checked: true } = status
    {
        Background::Color(theme.accent)
    } else {
        Background::Color(Color::TRANSPARENT)
    };

    checkbox::Style {
        background,
        icon_color: theme.crust,
        border,
        text_color: None
    }
});

catalog!(pane_grid, |theme| {
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(theme.accent.scale_alpha(ALPHA)),
            border: Border::default().color(theme.accent).rounded(BORDER_RADIUS)
        },
        picked_split: pane_grid::Line {
            color: theme.surface0,
            width: BORDER_WIDTH
        },
        hovered_split: pane_grid::Line {
            color: theme.accent,
            width: BORDER_WIDTH
        }
    }
});

catalog!(button, button::Status, |theme, status| {
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: theme.text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false
        },
        button::Status::Hovered => button::Style {
            background: Some(theme.overlay0.into()),
            text_color: theme.text,
            border: Border::default().rounded(BORDER_RADIUS),
            shadow: iced_helpers::shadow(*theme),
            snap: false
        },
        button::Status::Pressed => button::Style {
            background: Some(theme.accent.into()),
            text_color: theme.crust,
            border: Border::default().rounded(BORDER_RADIUS),
            shadow: iced_helpers::shadow(*theme),
            snap: false
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: theme.overlay0,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false
        }
    }
});

catalog!(scrollable, scrollable::Status, |theme, status| {
    let rail_active = Rail {
        background: Some(theme.crust.into()),
        border: border::rounded(BORDER_RADIUS),
        scroller: scrollable::Scroller {
            background: theme.overlay0.into(),
            border: border::rounded(BORDER_RADIUS)
        }
    };

    let mut rail_hovered = rail_active;
    rail_hovered.scroller.background = theme.overlay1.into();

    let mut rail_dragged = rail_active;
    rail_dragged.scroller.background = theme.accent.into();

    let auto_scroll = AutoScroll {
        background: theme.base.scale_alpha(0.9).into(),
        border: border::rounded(u32::MAX)
            .width(1)
            .color(theme.accent.scale_alpha(0.8)),
        shadow: iced_helpers::shadow(*theme),
        icon: theme.text.scale_alpha(0.8)
    };

    match status {
        scrollable::Status::Active { .. } => scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail_active,
            horizontal_rail: rail_active,
            gap: None,
            auto_scroll
        },
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
            ..
        } => scrollable::Style {
            container: container::Style::default(),
            vertical_rail: if is_vertical_scrollbar_hovered {
                rail_hovered
            } else {
                rail_active
            },
            horizontal_rail: if is_horizontal_scrollbar_hovered {
                rail_hovered
            } else {
                rail_active
            },
            gap: None,
            auto_scroll
        },
        scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
            ..
        } => scrollable::Style {
            container: container::Style::default(),
            vertical_rail: if is_vertical_scrollbar_dragged {
                rail_dragged
            } else {
                rail_active
            },
            horizontal_rail: if is_horizontal_scrollbar_dragged {
                rail_dragged
            } else {
                rail_active
            },
            gap: None,
            auto_scroll
        }
    }
});
impl ThemeConfig {
    pub fn theme(&self) -> Theme {
        let flavor_colors = match self.flavor {
            Flavor::Mocha => PALETTE.mocha,
            Flavor::Macchiato => PALETTE.macchiato,
            Flavor::Frappe => PALETTE.frappe,
            Flavor::Latte => PALETTE.latte
        }
        .colors;
        Theme {
            flavor: self.flavor,
            accent: match self.accent {
                Accent::Rosewater => flavor_colors.rosewater.to_iced(),
                Accent::Flamingo => flavor_colors.flamingo.to_iced(),
                Accent::Pink => flavor_colors.pink.to_iced(),
                Accent::Mauve => flavor_colors.mauve.to_iced(),
                Accent::Red => flavor_colors.red.to_iced(),
                Accent::Maroon => flavor_colors.maroon.to_iced(),
                Accent::Peach => flavor_colors.peach.to_iced(),
                Accent::Yellow => flavor_colors.yellow.to_iced(),
                Accent::Green => flavor_colors.green.to_iced(),
                Accent::Teal => flavor_colors.teal.to_iced(),
                Accent::Sky => flavor_colors.sky.to_iced(),
                Accent::Sapphire => flavor_colors.sapphire.to_iced(),
                Accent::Blue => flavor_colors.blue.to_iced(),
                Accent::Lavender => flavor_colors.lavender.to_iced()
            },
            rainbow: [
                flavor_colors.mauve.to_iced(),
                flavor_colors.blue.to_iced(),
                flavor_colors.green.to_iced(),
                flavor_colors.yellow.to_iced(),
                flavor_colors.peach.to_iced(),
                flavor_colors.red.to_iced()
            ],
            text: flavor_colors.text.to_iced(),
            subtext1: flavor_colors.subtext1.to_iced(),
            subtext0: flavor_colors.subtext0.to_iced(),
            overlay2: flavor_colors.overlay2.to_iced(),
            overlay1: flavor_colors.overlay1.to_iced(),
            overlay0: flavor_colors.overlay0.to_iced(),
            surface2: flavor_colors.surface2.to_iced(),
            surface1: flavor_colors.surface1.to_iced(),
            surface0: flavor_colors.surface0.to_iced(),
            base: flavor_colors.base.to_iced(),
            mantle: flavor_colors.mantle.to_iced(),
            crust: flavor_colors.crust.to_iced(),
            danger: flavor_colors.red.to_iced(),
            warning: flavor_colors.yellow.to_iced(),
            success: flavor_colors.green.to_iced(),
            bold: flavor_colors.sky.to_iced(),
            bold_italic: flavor_colors.sapphire.to_iced(),
            italic: flavor_colors.green.to_iced(),
            link_internal: flavor_colors.blue.to_iced(),
            link_external: flavor_colors.teal.to_iced()
        }
    }
}

trait ToIced {
    fn to_iced(self) -> Color;
}

impl ToIced for catppuccin::Color {
    fn to_iced(self) -> Color {
        let Rgb { r, g, b } = self.rgb;
        Color::from_rgb8(r, g, b)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeConfig {
    flavor: Flavor,
    accent: Accent
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
enum Flavor {
    Mocha,
    Macchiato,
    Frappe,
    Latte
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
enum Accent {
    Rosewater,
    Flamingo,
    Pink,
    Mauve,
    Red,
    Maroon,
    Peach,
    Yellow,
    Green,
    Teal,
    Sky,
    Sapphire,
    Blue,
    Lavender
}
