use std::{
    mem,
    ops::{Index, IndexMut, Range},
};

mod branch;
mod dynamic;
mod node;
mod raycast;

pub use branch::*;
pub use dynamic::*;
pub use node::*;
pub use raycast::*;

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};

use crate::generate::Generate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Octree {
    pub nodes: Vec<Node>,
    pub free_branches: Vec<u32>,
}

impl Default for Octree {
    fn default() -> Self {
        Self::new()
    }
}

impl Octree {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::empty()],
            free_branches: Vec::new(),
        }
    }

    pub const fn root(&self) -> u32 {
        0
    }

    pub fn push_branch(&mut self) -> u32 {
        if let Some(i) = self.free_branches.pop() {
            self[i..i + 8].fill(Node::empty());

            return i;
        }

        let index = self.len();
        self.extend(&[Node::empty(); 8]);

        index
    }

    pub fn remove_branch(&mut self, index: u32) {
        if index == self.len() - 8 {
            self.nodes.truncate(index as usize);
        } else {
            self.free_branches.push(index);
        }
    }

    pub fn generate<T: Generate>(sdf: &T) -> Self {
        let mut octree = Self::new();

        let dimensions = sdf.dimensions().as_ivec3();
        let depth = sdf.depth();

        for ix in -dimensions.x..dimensions.x {
            for iy in -dimensions.y..dimensions.y {
                for iz in -dimensions.z..dimensions.z {
                    let x = ix as f32 + 0.5;
                    let y = iy as f32 + 0.5;
                    let z = iz as f32 + 0.5;

                    let point = Vec3::new(x, y, z) / dimensions.as_vec3();

                    if let Some(node) = sdf.sdf(point) {
                        let branch = Branch::new(IVec3::new(ix, iy, iz), depth);
                        octree.set(branch, node);
                    }
                }
            }
        }

        octree
    }

    pub fn extend(&mut self, nodes: &[Node]) -> u32 {
        let index = self.nodes.len() as u32;
        self.nodes.extend_from_slice(nodes);
        index
    }

    pub fn iter_nodes(&self) -> NodeIterator<'_> {
        NodeIterator::new(self)
    }

    pub fn len(&self) -> u32 {
        self.nodes.len() as u32
    }

    pub fn size(&self) -> usize {
        self.nodes.len() * mem::size_of::<Node>()
    }

    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.nodes)
    }
}

impl Index<u32> for Octree {
    type Output = Node;

    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        &self.nodes[index as usize]
    }
}

impl IndexMut<u32> for Octree {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.nodes[index as usize]
    }
}

impl Index<Range<u32>> for Octree {
    type Output = [Node];

    #[inline]
    fn index(&self, index: Range<u32>) -> &Self::Output {
        &self.nodes[index.start as usize..index.end as usize]
    }
}

impl IndexMut<Range<u32>> for Octree {
    #[inline]
    fn index_mut(&mut self, index: Range<u32>) -> &mut Self::Output {
        &mut self.nodes[index.start as usize..index.end as usize]
    }
}

pub struct NodeIterator<'a> {
    octree: &'a Octree,
    stack: Vec<(Branch, u32)>,
}

impl<'a> NodeIterator<'a> {
    pub fn new(octree: &'a Octree) -> Self {
        Self {
            octree,
            stack: vec![(Branch::root(), octree.root())],
        }
    }
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = (Branch, &'a Node);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((branch, index)) = self.stack.pop() {
            let node = &self.octree[index];

            if node.is_empty() {
                continue;
            }

            if node.is_parent() {
                let pointer = node.pointer();

                for child in 0..8 {
                    let branch = branch.with_child(child);
                    self.stack.push((branch, pointer + child));
                }

                continue;
            }

            return Some((branch, node));
        }

        None
    }
}

macro_rules! impl_octree {
    ($ty:ty) => {
        impl $ty {
            pub fn set(&mut self, branch: impl Into<Branch>, node: Node) {
                let branch = branch.into();
                let mut parent = self.root();

                let mut stack = [0; 32];
                let mut stack_len = 0;

                // traverse down the tree until we reach the leaf
                for depth in 0..branch.depth {
                    let node = self[parent];

                    // push the stack
                    stack[stack_len] = parent;
                    stack_len += 1;

                    // if the node is not a parent, we need to split it
                    if !node.is_parent() {
                        let new_branch = self.push_branch();

                        // copy the old node to the new branch
                        if node.is_solid() {
                            for child in 0..8 {
                                self[new_branch + child] = node;
                            }
                        }

                        // replace the old node with a parent node
                        self[parent] = Node::parent(new_branch);
                    }

                    let pointer = self[parent].pointer();
                    let child = branch.child(depth);
                    parent = pointer + child;
                }

                self[parent] = node;

                // traverse back up the tree and combine leaf nodes
                for i in (0..stack_len).rev() {
                    let parent = stack[i];
                    let pointer = self[parent].pointer();

                    let mut combine = true;
                    for child in 0..8 {
                        combine &= self[pointer + child] == node;
                    }

                    if combine {
                        self[parent] = node;
                        self.remove_branch(pointer);
                    }
                }
            }

            pub fn remove(&mut self, branch: impl Into<Branch>) {
                let branch = branch.into();
                let mut parent = self.root();

                for depth in 0..branch.depth {
                    let node = self[parent];

                    if node.is_empty() {
                        return;
                    }

                    if node.is_solid() {
                        let new_branch = self.push_branch();

                        for child in 0..8 {
                            self[new_branch + child] = node;
                        }

                        self[parent] = Node::parent(new_branch);

                        let child = branch.child(depth);
                        parent = new_branch + child;
                        continue;
                    }

                    let pointer = node.pointer();

                    let mut chilren_empty = true;
                    for child in 0..8 {
                        chilren_empty &= self[pointer + child].is_empty();
                    }

                    if chilren_empty {
                        self[parent] = Node::empty();
                        self.remove_branch(pointer);
                        return;
                    }

                    let child = branch.child(depth);
                    parent = pointer + child;
                }

                self[parent] = Node::empty();
            }

            pub fn join(&mut self, branch: impl Into<Branch>, depth: u32, other: &Octree) {
                let branch = branch.into();

                for (other_branch, node) in other.iter_nodes() {
                    let mut other_branch = other_branch;
                    other_branch.depth += depth;

                    let offset = other_branch.depth as i32 - branch.depth as i32 - depth as i32;

                    if offset >= 0 {
                        other_branch.path += branch.path << offset;
                        self.set(other_branch, *node);

                        continue;
                    }

                    let half = 1 << -offset;

                    for x in 0..half {
                        for y in 0..half {
                            for z in 0..half {
                                let mut other_branch = other_branch;
                                other_branch.path = other_branch.path << -offset;
                                other_branch.path += branch.path;
                                other_branch.path += IVec3::new(x, y, z);
                                other_branch.depth -= offset as u32;
                                self.set(other_branch, *node);
                            }
                        }
                    }
                }
            }

            pub fn difference(&mut self, branch: impl Into<Branch>, depth: u32, other: &Octree) {
                let branch = branch.into();

                for (other_branch, _) in other.iter_nodes() {
                    let mut other_branch = other_branch;
                    other_branch.depth += depth;

                    let offset = other_branch.depth as i32 - branch.depth as i32 - depth as i32;

                    if offset >= 0 {
                        other_branch.path += branch.path << offset;
                        self.remove(other_branch);

                        continue;
                    }

                    let half = 1 << -offset;

                    for x in 0..half {
                        for y in 0..half {
                            for z in 0..half {
                                let mut other_branch = other_branch;
                                other_branch.path = other_branch.path << -offset;
                                other_branch.path += branch.path;
                                other_branch.path += IVec3::new(x, y, z);
                                other_branch.depth -= offset as u32;
                                self.remove(other_branch);
                            }
                        }
                    }
                }
            }
        }
    };
}

impl_octree!(Octree);
impl_octree!(DynamicOctree);
