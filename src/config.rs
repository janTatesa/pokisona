use std::{collections::HashMap, fs, path::Path, str::FromStr};

use color_eyre::eyre::Result;
use figment::{
    Figment,
    providers::{Data, Toml}
};
use iced::{
    advanced::graphics::core::keyboard,
    keyboard::{Key, Modifiers, key::Named}
};
use serde::{Deserialize, Deserializer, de};

use crate::{command::Command, theme::Theme};

#[derive(Deserialize, Clone)]
pub struct Config {
    pub theme: Theme,
    pub scale: ScaleConfig,
    pub keybindings: HashMap<Keybinding, Command>
}

impl Config {
    const DEFAULT: &str = include_str!("./default-config.toml");
    pub fn new(path: &Path, use_default_config: bool) -> Result<Self> {
        if !path.exists() {
            fs::write(path, Self::DEFAULT)?;
        }

        let default = Data::<Toml>::string(Self::DEFAULT);
        let config = Data::<Toml>::file_exact(path);
        let mut figment = Figment::from(default);
        if !use_default_config {
            figment = figment.merge(config)
        }

        Ok(figment.extract()?)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    Delete,
    Insert,
    Char(char),
    Esc
}

struct KeyCodeNotFound;

impl FromStr for KeyCode {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "minus" => Self::Char('-'),
            "backspace" => Self::Backspace,
            "ret" | "enter" | "return" => Self::Enter,
            "left" => Self::Left,
            "right" => Self::Right,
            "up" => Self::Up,
            "down" => Self::Down,
            "home" => Self::Home,
            "end" => Self::End,
            "pageup" | "pgup" => Self::PageUp,
            "pagedown" | "pgdn" => Self::PageDown,
            "tab" => Self::Tab,
            "del" | "delete" => Self::Delete,
            "ins" | "insert" => Self::Insert,
            "esc" => Self::Esc,
            char if char.len() == 1 => Self::Char(char.chars().next().unwrap()),
            _ => return Err(KeyCodeNotFound)
        })
    }

    type Err = KeyCodeNotFound;
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Keybinding {
    key: KeyCode,
    modifiers: Modifiers
}

impl Keybinding {
    pub fn from_iced_key_event(key: keyboard::Event) -> Option<Self> {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = key else {
            return None;
        };

        let key = match key {
            Key::Named(Named::Backspace) => KeyCode::Backspace,
            Key::Named(Named::Enter) => KeyCode::Enter,
            Key::Named(Named::ArrowLeft) => KeyCode::Left,
            Key::Named(Named::ArrowRight) => KeyCode::Right,
            Key::Named(Named::ArrowUp) => KeyCode::Up,
            Key::Named(Named::ArrowDown) => KeyCode::Down,
            Key::Named(Named::PageUp) => KeyCode::PageUp,
            Key::Named(Named::PageDown) => KeyCode::PageDown,
            Key::Named(Named::Tab) => KeyCode::Tab,
            Key::Named(Named::Delete) => KeyCode::Delete,
            Key::Named(Named::Insert) => KeyCode::Insert,
            Key::Named(Named::Escape) => KeyCode::Esc,
            Key::Character(char) if char.len() == 1 => KeyCode::Char(char.chars().next().unwrap()),
            _ => return None
        };

        Some(Self { key, modifiers })
    }
}

impl<'de> Deserialize<'de> for Keybinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let string = String::deserialize(deserializer)?;
        if string == "minus" {
            return Ok(Self {
                key: KeyCode::Char('-'),
                modifiers: Modifiers::empty()
            });
        }

        let mut args = string.split('-');
        let code = args
            .next_back()
            .ok_or_else(|| de::Error::custom("Empty keybinding"))?;

        let key = KeyCode::from_str(code).map_err(|_| de::Error::custom("Invalid key code"))?;
        let mut modifiers = Modifiers::empty();
        for arg in args {
            modifiers |= match arg {
                "C" => Modifiers::CTRL,
                "S" => Modifiers::SHIFT,
                "A" => Modifiers::ALT,
                _ => return Err(de::Error::custom("Invalid modifier"))
            }
        }

        Ok(Self { key, modifiers })
    }
}

#[derive(Deserialize, Clone, Copy)]
pub struct ScaleConfig {
    pub default: f32,
    pub default_step: f32
}
