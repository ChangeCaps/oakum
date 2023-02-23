use crate::{app::UpdateContext, octree::DynamicOctree, render::Camera};

pub struct World {
    pub camera: Camera,
    pub octree: DynamicOctree,
}

impl World {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            octree: DynamicOctree::empty(),
        }
    }

    pub fn update(&mut self, cx: UpdateContext) {
        self.camera.update(cx);
    }

    pub fn post_update(&mut self) {
        self.octree.clear_segments();
    }
}
