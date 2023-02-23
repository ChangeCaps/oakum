use glam::{IVec3, Mat4, Vec3};

use crate::ray::Ray;

use super::{Branch, Octree};

#[derive(Clone, Copy, Debug)]
pub struct OctreeHit {
    pub index: u32,
    pub branch: Branch,
    pub distance: f32,
    pub point: Vec3,
    pub normal: IVec3,
}

fn in_bounds(point: Vec3) -> bool {
    point.abs().cmple(Vec3::ONE).all()
}

fn project(origin: Vec3, direction: Vec3) -> Option<Vec3> {
    if in_bounds(origin) {
        return Some(origin);
    }

    let tmin = (Vec3::NEG_ONE - origin) / direction;
    let tmax = (Vec3::ONE - origin) / direction;

    let near = tmin.min(tmax).max_element();
    let far = tmin.max(tmax).min_element();

    if near > far || far < 0.0 {
        return None;
    }

    Some(origin + direction * near)
}

fn split(path: IVec3, depth: u32) -> Vec3 {
    let scale = 1 << depth;
    (path.as_vec3() + Vec3::splat(0.5)) / scale as f32 - 1.0
}

fn select_initial_child(point: Vec3) -> u32 {
    let mut child = 0;

    if point.x >= 0.0 {
        child |= 1;
    }

    if point.y >= 0.0 {
        child |= 2;
    }

    if point.z >= 0.0 {
        child |= 4;
    }

    child
}

fn select_child(point: Vec3, path: IVec3, depth: u32) -> u32 {
    let split = split(path, depth);
    let mut child = 0;

    if point.x >= split.x {
        child |= 1;
    }

    if point.y >= split.y {
        child |= 2;
    }

    if point.z >= split.z {
        child |= 4;
    }

    child
}

fn add_child(path: IVec3, child: u32) -> IVec3 {
    let mut path: IVec3 = path << 1;

    if child & 1 != 0 {
        path.x |= 1;
    }

    if child & 2 != 0 {
        path.y |= 1;
    }

    if child & 4 != 0 {
        path.z |= 1;
    }

    path
}

fn extract_child(path: IVec3, depth: u32) -> u32 {
    let mut child = 0;

    if path.x & (1 << depth) != 0 {
        child |= 1;
    }

    if path.y & (1 << depth) != 0 {
        child |= 2;
    }

    if path.z & (1 << depth) != 0 {
        child |= 4;
    }

    child
}

impl Octree {
    pub fn raycast(&self, transform: Mat4, ray: Ray) -> Option<OctreeHit> {
        let ray = ray.transform(transform.inverse());
        let hit = self.raycast_normalized(ray)?;

        let position = transform.transform_point3(hit.point);
        Some(OctreeHit {
            index: hit.index,
            branch: hit.branch,
            distance: (position - ray.origin).length(),
            point: position,
            normal: hit.normal,
        })
    }

    pub fn raycast_normalized(&self, ray: Ray) -> Option<OctreeHit> {
        let mut point = project(ray.origin, ray.direction)?;
        let direction = ray.direction.normalize();

        let side_axis = point.abs().cmpge(Vec3::ONE);
        let side_sign = point.signum().as_ivec3();
        let mut normal = IVec3::select(side_axis, side_sign, IVec3::ZERO);
        let dir = direction.signum().as_ivec3();

        let root = self[self.root()];
        if root.is_empty() {
            return None;
        }
        if root.is_solid() {
            return Some(OctreeHit {
                index: self.root(),
                branch: Branch::root(),
                distance: 0.0,
                point,
                normal,
            });
        }

        let mut parent = root.pointer();
        let mut depth = 0;
        let mut child = select_initial_child(point);
        let mut path = add_child(IVec3::ZERO, child);
        let mut stack = [0; 32];
        stack[0] = parent;

        loop {
            let node = self[parent + child];

            if node.is_parent() {
                parent = node.pointer();
                child = select_child(point, path, depth);
                path = add_child(path, child);

                depth += 1;
                stack[depth as usize] = parent;
                continue;
            }

            if node.is_solid() {
                let half = 1 << depth;
                let branch = Branch::new(path - half, depth + 1);

                let hit = OctreeHit {
                    index: parent + child,
                    branch,
                    distance: (point - point).length(),
                    point: point + direction * 0.0001,
                    normal,
                };

                return Some(hit);
            }

            let old_path = path;
            let split = split(path, depth);
            let bounds = split + dir.as_vec3() / (1 << depth + 1) as f32;
            let t = (bounds - point) / direction;

            let tmin = t.min_element();
            if tmin == t.x {
                path.x += dir.x;
                normal = IVec3::new(-dir.x, 0, 0);
            } else if tmin == t.y {
                path.y += dir.y;
                normal = IVec3::new(0, -dir.y, 0);
            } else {
                path.z += dir.z;
                normal = IVec3::new(0, 0, -dir.z);
            }

            point += direction * tmin;

            let path_diff = path ^ old_path;
            let diff = path_diff.x | path_diff.y | path_diff.z;
            let flip = 31 - diff.leading_zeros() as u32;

            if flip > depth {
                return None;
            }

            parent = stack[depth as usize - flip as usize];
            child = extract_child(path, flip);

            for i in (1..=flip).rev() {
                let node = self[parent + child];
                if !node.is_parent() {
                    depth -= i;
                    path = path >> i;
                    break;
                }

                let i = i - 1;
                parent = node.pointer();
                child = extract_child(path, i);
                stack[depth as usize - i as usize] = parent;
            }
        }
    }
}
