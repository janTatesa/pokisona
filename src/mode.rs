use iced::{
    Color,
    keyboard::{Key, Modifiers, key::Named},
    widget::text_editor::{self, Binding, KeyPress, Motion}
};
use strum::AsRefStr;

use crate::{CATPPUCCIN_MOCHA, Message, command::Command};

#[derive(Clone, Copy, AsRefStr)]
pub enum Mode {
    Normal,
    Select,
    Insert
}

impl Mode {
    pub fn bindings(&self) -> fn(KeyPress) -> Option<Binding<Message>> {
        match self {
            Mode::Normal => normal,
            Mode::Select => select,
            Mode::Insert => insert
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Mode::Normal => CATPPUCCIN_MOCHA.mauve,
            Mode::Select => CATPPUCCIN_MOCHA.flamingo,
            Mode::Insert => CATPPUCCIN_MOCHA.green
        }
        .into()
    }
}

fn universal(key: Key<&str>, modifiers: Modifiers) -> Option<Binding<Message>> {
    use Binding as B;
    use Key as K;
    use Modifiers as M;
    use Named as N;
    let binding = match (key, modifiers) {
        (K::Character("c"), M::CTRL) => B::Copy,
        (K::Character("v"), M::CTRL) => B::Paste,
        (K::Character("x"), M::CTRL) => B::Cut,
        (K::Character("s"), M::CTRL) => B::Custom(Message::Command(Command::Write(None))),
        (K::Character("a"), M::NONE) => B::SelectAll,
        (K::Named(N::PageDown), M::NONE) => B::Move(Motion::PageDown),
        (K::Named(N::PageUp), M::NONE) => B::Move(Motion::PageUp),
        (K::Named(N::Home), M::NONE) => B::Move(Motion::Home),
        (K::Named(N::End), M::NONE) => B::Move(Motion::End),
        (K::Named(N::Home), M::SHIFT) => B::Select(Motion::Home),
        (K::Named(N::End), M::SHIFT) => B::Select(Motion::End),
        (K::Named(N::ArrowLeft), M::NONE) => B::Move(Motion::Left),
        (K::Named(N::ArrowRight), M::NONE) => B::Move(Motion::Right),
        (K::Named(N::ArrowUp), M::NONE) => B::Move(Motion::Up),
        (K::Named(N::ArrowDown), M::NONE) => B::Move(Motion::Down),
        (K::Named(N::ArrowLeft), M::SHIFT) => B::Select(Motion::Left),
        (K::Named(N::ArrowRight), M::SHIFT) => B::Select(Motion::Right),
        (K::Named(N::ArrowUp), M::SHIFT) => B::Select(Motion::Up),
        (K::Named(N::ArrowDown), M::SHIFT) => B::Select(Motion::Down),
        (K::Named(N::ArrowLeft), _) if modifiers == M::SHIFT | M::CTRL => {
            B::Select(Motion::WordLeft)
        }
        (K::Named(N::ArrowRight), _) if modifiers == M::SHIFT | M::CTRL => {
            B::Select(Motion::WordRight)
        }
        _ => return None
    };

    Some(binding)
}

fn normal(key: KeyPress) -> Option<Binding<Message>> {
    if key.status == text_editor::Status::Active {
        return None;
    }

    let KeyPress { key, modifiers, .. } = key;

    use Binding as B;
    use Key as K;
    use Modifiers as M;
    let binding = match (key.as_ref(), modifiers) {
        (K::Character(";"), M::SHIFT) => B::Custom(Message::EnterCommandMode),
        (K::Character("i"), M::NONE) => B::Custom(Message::SwitchMode(Mode::Insert)),
        (K::Character("v"), M::NONE) => B::Custom(Message::SwitchMode(Mode::Select)),
        (K::Character("h"), M::NONE) => B::Move(Motion::Left),
        (K::Character("j"), M::NONE) => B::Move(Motion::Down),
        (K::Character("k"), M::NONE) => B::Move(Motion::Up),
        (K::Character("l"), M::NONE) => B::Move(Motion::Right),
        (K::Character("d"), M::NONE) => B::Delete,
        (K::Character("w"), M::NONE) => B::Move(Motion::WordLeft),
        (K::Character("b"), M::NONE) => B::Move(Motion::WordRight),
        (K::Character("y"), M::NONE) => B::Copy,
        (K::Character("p"), M::NONE) => B::Paste,
        (K::Character("%"), M::NONE) => B::SelectAll,
        (K::Character("x"), M::NONE) => B::SelectLine,
        (key, _) => return universal(key, modifiers)
    };

    Some(binding)
}

fn insert(key: KeyPress) -> Option<Binding<Message>> {
    if !matches!(key.status, text_editor::Status::Focused { .. }) {
        return None;
    }

    let KeyPress {
        key,
        modified_key,
        modifiers,
        ..
    } = key;

    use Binding as B;
    use Key as K;
    use Modifiers as M;
    use Named as N;
    let binding = match (key.as_ref(), modifiers) {
        (K::Named(N::Escape), M::NONE) => B::Custom(Message::SwitchMode(Mode::Normal)),
        (K::Named(N::Enter), M::NONE) => B::Enter,
        (K::Named(N::Delete), M::NONE) => B::Delete,
        (K::Named(N::Backspace), M::NONE) => B::Backspace,
        (K::Named(N::Space), M::NONE) => B::Insert(' '),
        (K::Character(ch), M::NONE) => B::Sequence(ch.chars().map(B::Insert).collect()),
        (K::Character(_), _) if modified_key != key && !modifiers.contains(M::CTRL) => {
            let bindings = if let K::Character(ch) = modified_key {
                ch.chars().map(B::Insert).collect()
            } else {
                return None;
            };

            B::Sequence(bindings)
        }
        (key, _) => return universal(key, modifiers)
    };

    Some(binding)
}

fn select(key: KeyPress) -> Option<Binding<Message>> {
    if key.status == text_editor::Status::Active {
        return None;
    }

    let KeyPress { key, modifiers, .. } = key;

    use Binding as B;
    use Key as K;
    use Modifiers as M;
    use Named as N;
    let binding = match (key.as_ref(), modifiers) {
        (K::Character(";"), M::SHIFT) => B::Custom(Message::EnterCommandMode),
        (K::Named(N::Escape), M::NONE) => B::Custom(Message::SwitchMode(Mode::Normal)),
        (K::Character("i"), M::NONE) => B::Custom(Message::SwitchMode(Mode::Insert)),
        (K::Character("h") | K::Named(N::ArrowLeft), M::NONE) => B::Select(Motion::Left),
        (K::Character("j") | K::Named(N::ArrowDown), M::NONE) => B::Select(Motion::Down),
        (K::Character("k") | K::Named(N::ArrowUp), M::NONE) => B::Select(Motion::Up),
        (K::Character("l") | K::Named(N::ArrowRight), M::NONE) => B::Select(Motion::Right),
        (K::Character("d"), M::NONE) => B::Delete,
        (K::Character("w"), M::NONE) => B::Select(Motion::WordLeft),
        (K::Character("b"), M::NONE) => B::Select(Motion::WordRight),
        (K::Character("y"), M::NONE) => B::Copy,
        (K::Character("p"), M::NONE) => B::Paste,
        (K::Character("%"), M::NONE) => B::SelectAll,
        (K::Character("x"), M::NONE) => B::SelectLine,
        (key, _) => return universal(key, modifiers)
    };

    Some(binding)
}
