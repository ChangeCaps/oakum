use glam::{UVec3, Vec3};

use crate::octree::Node;

use super::Generate;

#[derive(Clone, Copy, Debug, Default)]
pub struct Sphere {
    pub radius: u32,
    pub depth: u32,
}

impl Sphere {
    pub const fn new(radius: u32, depth: u32) -> Self {
        Self { radius, depth }
    }
}

impl Generate for Sphere {
    fn dimensions(&self) -> UVec3 {
        UVec3::splat(self.radius)
    }

    fn depth(&self) -> u32 {
        self.depth
    }

    fn get_node(&self, point: Vec3) -> Option<Node> {
        if point.length() < 1.0 {
            Some(Node::solid(255, 255, 255))
        } else {
            None
        }
    }
}
