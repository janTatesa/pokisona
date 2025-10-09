// TODO: Integrate this with iced
use catppuccin::{PALETTE, Rgb};
use iced::Color;

const fn catppuccin_color_to_iced_color(catppuccin: catppuccin::Color) -> Color {
    let Rgb { r, g, b } = catppuccin.rgb;
    Color::from_rgb8(r, g, b)
}

pub const MANTLE: Color = catppuccin_color_to_iced_color(PALETTE.mocha.colors.mantle);
pub const CRUST: Color = catppuccin_color_to_iced_color(PALETTE.mocha.colors.crust);
pub const ACCENT: Color = catppuccin_color_to_iced_color(PALETTE.mocha.colors.mauve);
pub const OVERLAY0: Color = catppuccin_color_to_iced_color(PALETTE.mocha.colors.overlay0);
