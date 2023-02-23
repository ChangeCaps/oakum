use deref_derive::{Deref, DerefMut};
use glam::{IVec3, Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deref, DerefMut)]
pub struct Branch {
    #[deref]
    pub path: IVec3,
    pub depth: u32,
}

impl Branch {
    pub const fn new(path: IVec3, depth: u32) -> Self {
        Self { path, depth }
    }

    pub const fn root() -> Self {
        Self {
            path: IVec3::ZERO,
            depth: 0,
        }
    }

    pub fn from_point(transform: Mat4, point: Vec3, depth: u32) -> Self {
        let point = transform.inverse().transform_point3(point);
        Self::from_point_normalized(point, depth)
    }

    pub fn from_point_normalized(point: Vec3, depth: u32) -> Self {
        let half = 1 << (depth - 1);
        let path = IVec3::new(
            (point.x * half as f32 - 0.5).round() as i32,
            (point.y * half as f32 - 0.5).round() as i32,
            (point.z * half as f32 - 0.5).round() as i32,
        );

        Self { path, depth }
    }

    const fn is_positive(&self, depth: u32, axis: i32) -> bool {
        let half = (1 << (self.depth - 1)) as i32;
        let mask = 1 << (self.depth - depth - 1);
        let absolute = axis + half;
        absolute & mask != 0
    }

    pub const fn is_x_positive(&self, depth: u32) -> bool {
        self.is_positive(depth, self.path.x)
    }

    pub const fn is_y_positive(&self, depth: u32) -> bool {
        self.is_positive(depth, self.path.y)
    }

    pub const fn is_z_positive(&self, depth: u32) -> bool {
        self.is_positive(depth, self.path.z)
    }

    pub const fn child(&self, depth: u32) -> u32 {
        let mut child = 0;

        if self.is_x_positive(depth) {
            child |= 1;
        }

        if self.is_y_positive(depth) {
            child |= 2;
        }

        if self.is_z_positive(depth) {
            child |= 4;
        }

        child
    }

    pub const fn with_child(&self, child: u32) -> Self {
        let mut branch = Branch {
            path: IVec3::new(self.path.x * 2, self.path.y * 2, self.path.z * 2),
            depth: self.depth + 1,
        };

        if child & 1 != 0 && self.depth > 0 {
            branch.path.x += 1;
        } else if child & 1 == 0 && self.depth == 0 {
            branch.path.x = -1;
        }

        if child & 2 != 0 && self.depth > 0 {
            branch.path.y += 1;
        } else if child & 2 == 0 && self.depth == 0 {
            branch.path.y = -1;
        }

        if child & 4 != 0 && self.depth > 0 {
            branch.path.z += 1;
        } else if child & 4 == 0 && self.depth == 0 {
            branch.path.z = -1;
        }

        branch
    }
}

impl From<(IVec3, u32)> for Branch {
    fn from((path, depth): (IVec3, u32)) -> Self {
        Self { path, depth }
    }
}

impl From<(i32, i32, i32, u32)> for Branch {
    fn from((x, y, z, depth): (i32, i32, i32, u32)) -> Self {
        Self {
            path: IVec3::new(x, y, z),
            depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_index() {
        assert_eq!(Branch::new(IVec3::new(-1, -1, -1), 1).child(0), 0);
        assert_eq!(Branch::new(IVec3::new(0, -1, -1), 1).child(0), 1);
        assert_eq!(Branch::new(IVec3::new(-1, 0, -1), 1).child(0), 2);
        assert_eq!(Branch::new(IVec3::new(0, 0, -1), 1).child(0), 3);
        assert_eq!(Branch::new(IVec3::new(-1, -1, 0), 1).child(0), 4);
        assert_eq!(Branch::new(IVec3::new(0, -1, 0), 1).child(0), 5);
        assert_eq!(Branch::new(IVec3::new(-1, 0, 0), 1).child(0), 6);
        assert_eq!(Branch::new(IVec3::new(0, 0, 0), 1).child(0), 7);
    }
}
