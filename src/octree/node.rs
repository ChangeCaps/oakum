use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Node {
    pub flags: u32,
    pub data: u32,
}

impl Node {
    pub const SOLID_BIT: u32 = 1 << 0;
    pub const PARENT_BIT: u32 = 1 << 1;
    pub const SHADOW_BIT: u32 = 1 << 2;
    pub const EMPTY_MASK: u32 = Self::PARENT_BIT | Self::SOLID_BIT;

    pub const fn empty() -> Self {
        Self { flags: 0, data: 0 }
    }

    pub const fn solid(r: u8, g: u8, b: u8) -> Self {
        Self {
            flags: Self::SOLID_BIT | Self::SHADOW_BIT,
            data: ((b as u32) << 16) | ((g as u32) << 8) | ((r as u32) << 0),
        }
    }

    pub const fn translucent(r: u8, g: u8, b: u8) -> Self {
        Self {
            flags: Self::SOLID_BIT,
            data: ((b as u32) << 16) | ((g as u32) << 8) | ((r as u32) << 0),
        }
    }

    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::solid(r, g, b)
    }

    pub fn rgb(color: Vec3) -> Self {
        Self::solid(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
        )
    }

    pub const fn rgb8_translucent(r: u8, g: u8, b: u8) -> Self {
        Self::translucent(r, g, b)
    }

    pub fn rgb_translucent(color: Vec3) -> Self {
        Self::translucent(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
        )
    }

    pub const fn parent(pointer: u32) -> Self {
        Self {
            flags: Self::PARENT_BIT,
            data: pointer,
        }
    }

    pub const fn is_parent(&self) -> bool {
        self.flags & Self::PARENT_BIT != 0
    }

    pub const fn is_solid(&self) -> bool {
        self.flags & Self::SOLID_BIT != 0
    }

    pub const fn is_shadow(&self) -> bool {
        self.flags & Self::SHADOW_BIT != 0
    }

    pub const fn is_empty(&self) -> bool {
        self.flags & Self::EMPTY_MASK == 0
    }

    pub const fn pointer(&self) -> u32 {
        self.data
    }

    pub const fn r(&self) -> u8 {
        (self.data >> 0) as u8
    }

    pub const fn g(&self) -> u8 {
        (self.data >> 8) as u8
    }

    pub const fn b(&self) -> u8 {
        (self.data >> 16) as u8
    }

    pub fn set_parent(&mut self) {
        self.flags |= Self::PARENT_BIT;
    }

    pub fn set_pointer(&mut self, pointer: u32) {
        self.data = pointer;
    }

    pub fn set_solid(&mut self) {
        self.flags |= Self::SOLID_BIT;
    }

    pub fn set_empty(&mut self) {
        self.flags &= !Self::EMPTY_MASK;
    }
}
