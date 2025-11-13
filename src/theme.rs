use catppuccin::{PALETTE, Rgb};
use iced::{
    Background, Border, Color,
    theme::{Base, Mode, Palette, Style},
    widget::{
        checkbox, container,
        rule::{self, FillMode},
        text, text_input
    }
};
use iced_selection::text as selection_text;
use serde::Deserialize;

use crate::iced_helpers::{ALPHA, BORDER_RADIUS, BORDER_WIDTH};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    flavor: Flavor,
    pub accent: Color,
    /// Colors things like headings
    pub misc_colors: [Color; 6],
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
            background_color: self.mantle,
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
}

macro_rules! catalog {
    ($widget:ident, $style:expr) => {
        impl $widget::Catalog for Theme {
            type Class<'a> = $widget::StyleFn<'a, Self>;

            fn default<'a>() -> Self::Class<'a> {
                Box::new($style)
            }

            fn style(&self, class: &Self::Class<'_>) -> $widget::Style {
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

catalog!(container, |_| container::Style::default());
catalog!(text, |_| text::Style::default());
catalog!(selection_text, |theme| selection_text::Style {
    color: None,
    selection: theme.accent.scale_alpha(ALPHA)
});

catalog!(text_input, text_input::Status, |theme, _| {
    text_input::Style {
        background: Background::Color(theme.mantle),
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

catalog!(checkbox, checkbox::Status, default_checkbox_style);
// TODO: maybe behave differently when it's disabled
fn default_checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
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
}

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
            misc_colors: [
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
