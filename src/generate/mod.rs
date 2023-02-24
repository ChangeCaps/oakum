mod block;
mod shape;

pub use block::*;
pub use shape::*;

use std::cmp::Ordering;

use glam::{UVec3, Vec3};

use crate::octree::Node;

pub trait Generate {
    fn dimensions(&self) -> UVec3;
    fn depth(&self) -> u32;

    fn get_node(&self, point: Vec3) -> Option<Node>;
}

#[derive(Clone, Copy, Debug)]
pub struct Sdf {
    pub distance: f32,
    pub node: Node,
}

impl Sdf {
    pub const fn new(distance: f32, node: Node) -> Self {
        Self { distance, node }
    }
}

impl PartialEq for Sdf {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Sdf {}

impl PartialOrd for Sdf {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.distance.partial_cmp(&other.distance)
    }
}
