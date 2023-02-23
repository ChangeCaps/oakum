use glam::{UVec3, Vec3};
use noise::{NoiseFn, Perlin};

use crate::octree::Node;

use super::Generate;

pub fn perlin(p: Vec3) -> f32 {
    let noise = Perlin::new(0);
    noise.get([p.x as f64, p.y as f64, p.z as f64]) as f32
}

pub fn sperlin(p: Vec3) -> f32 {
    let noise = Perlin::new(0);
    noise.get([p.x as f64, p.y as f64, p.z as f64]) as f32 * 0.5 + 0.5
}

pub struct GrassBlock;

impl Generate for GrassBlock {
    fn dimensions(&self) -> UVec3 {
        UVec3::splat(16)
    }

    fn depth(&self) -> u32 {
        6
    }

    fn sdf(&self, point: Vec3) -> Option<Node> {
        let mut surface_offset = sperlin(point * Vec3::new(4.0, 6.0, 4.0)) * 0.2;
        let grass_offset = sperlin(point * Vec3::new(10.0, 0.0, 10.0)) * 0.5;

        let step_offset = sperlin(point * Vec3::new(10.0, 10.0, 10.0)) * 0.25;
        let step = f32::floor((point.y + step_offset) * 4.0) / 4.0;
        let mut color = Vec3::new(0.76 + step * 0.2, 0.48 + step * 0.15, 0.21 + step * 0.1);

        if sperlin(point * 8.0) > 0.8 {
            color = Vec3::splat(0.7);
        }

        if point.y > 0.5 + grass_offset {
            color = Vec3::new(0.34, 0.77, 0.26);
        } else {
            surface_offset += sperlin(point * 2.0) * 0.3;
            surface_offset += 2.0 / 16.0;
        }

        let xo = point.x.abs().powi(4);
        let zo = point.z.abs().powi(4);

        let mut yo = point.y.min(0.0).powi(4);
        yo += point.y.max(0.0).powi(64);

        let base = (xo + zo + yo).sqrt();

        if base > 1.0 - surface_offset {
            return None;
        }

        Some(Node::rgb(color))
    }
}
