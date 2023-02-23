use std::{collections::HashSet, hash::Hash};

use deref_derive::{Deref, DerefMut};
use glam::Vec2;
use winit::event::MouseButton;

#[derive(Clone, Debug)]
pub struct Input<T> {
    pub held: HashSet<T>,
    pub pressed: HashSet<T>,
    pub released: HashSet<T>,
}

impl<T> Input<T> {
    pub fn new() -> Self {
        Self {
            held: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> Input<T> {
    pub fn update(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }

    pub fn press(&mut self, key: T) {
        self.pressed.insert(key);
        self.held.insert(key);
    }

    pub fn release(&mut self, key: T) {
        self.released.insert(key);
        self.held.remove(&key);
    }

    pub fn is_held(&self, key: T) -> bool {
        self.held.contains(&key)
    }

    pub fn is_pressed(&self, key: T) -> bool {
        self.pressed.contains(&key)
    }

    pub fn is_released(&self, key: T) -> bool {
        self.released.contains(&key)
    }
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct Mouse {
    pub position: Vec2,
    pub delta: Vec2,
    pub scroll: Vec2,

    #[deref]
    pub input: Input<MouseButton>,
}

impl Mouse {
    pub fn update(&mut self) {
        self.delta = Vec2::ZERO;
        self.scroll = Vec2::ZERO;
        self.input.update();
    }
}

pub type Key = winit::event::VirtualKeyCode;

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct Keyboard {
    #[deref]
    pub input: Input<Key>,
}

impl Keyboard {
    pub fn update(&mut self) {
        self.input.update();
    }
}
