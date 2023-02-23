use glam::{Mat4, Vec3};

#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    pub fn transform(&self, transform: Mat4) -> Self {
        let origin = transform.transform_point3(self.origin);
        let direction = transform.transform_vector3(self.direction);

        Self { origin, direction }
    }
}
