use std::{
    mem,
    ops::{Index, IndexMut, Range},
};

use deref_derive::{Deref, DerefMut};

use super::{Node, Octree};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Segment {
    pub start: u32,
    pub len: u32,
}

impl Segment {
    pub const BATCH_THRESHOLD: u32 = 1024;

    pub const fn new(start: u32, length: u32) -> Self {
        Self { start, len: length }
    }

    pub const fn end(&self) -> u32 {
        self.start + self.len
    }

    pub const fn batch_end(&self) -> u32 {
        self.end() + Self::BATCH_THRESHOLD
    }

    pub const fn byte_start(&self) -> usize {
        self.start as usize * mem::size_of::<Node>()
    }

    pub const fn byte_len(&self) -> usize {
        self.len as usize * mem::size_of::<Node>()
    }

    pub const fn byte_end(&self) -> usize {
        self.end() as usize * mem::size_of::<Node>()
    }

    pub fn join(self, other: Self) -> Self {
        let start = u32::min(self.start, other.start);
        let end = u32::max(self.end(), other.end());

        Self {
            start,
            len: end - start,
        }
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct DynamicOctree {
    #[deref]
    octree: Octree,
    /// Segments that have changed since the last write.
    ///
    /// The are sorted by their start position,
    /// and overlapping segments are joined.
    segments: Vec<Segment>,
}

impl DynamicOctree {
    pub fn empty() -> Self {
        Self {
            octree: Octree::new(),
            segments: Vec::new(),
        }
    }

    pub fn new(octree: Octree) -> Self {
        let segment = Segment::new(0, octree.len() as u32);

        Self {
            octree,
            segments: vec![segment],
        }
    }

    pub fn push_branch(&mut self) -> u32 {
        let index = self.octree.push_branch();
        self.push_segment(Segment::new(index, 8));

        index
    }

    pub fn remove_branch(&mut self, index: u32) {
        self.octree.remove_branch(index);

        // ensure that the last segment is not larger than the octree
        if let Some(segment) = self.segments.last_mut() {
            if segment.byte_end() > self.octree.size() {
                segment.len -= 8;
            }
        }
    }
}

impl DynamicOctree {
    fn segment_before(&self, segment: Segment) -> Result<usize, usize> {
        (self.segments).binary_search_by_key(&segment.start, |s| s.start)
    }

    pub fn push_segment(&mut self, segment: Segment) {
        // find the segment that is before the new one
        let after = match self.segment_before(segment) {
            Ok(index) => {
                // the new segment starts at the same position as an existing one
                // -> join them
                self.segments[index] = self.segments[index].join(segment);

                index + 1
            }
            Err(i) => {
                // the new segment starts after an existing one
                // if the new segment overlaps with the next one
                // -> join them
                if i > 0 && self.segments[i - 1].batch_end() >= segment.start {
                    self.segments[i - 1] = self.segments[i - 1].join(segment);
                    i
                } else {
                    self.segments.insert(i, segment);
                    i + 1
                }
            }
        };

        // join all segments that overlap with the new one
        for _ in after..self.segments.len() {
            if self.segments[after].start >= segment.batch_end() {
                break;
            }

            self.segments[after - 1] = self.segments[after - 1].join(self.segments[after]);
            self.segments.remove(after);
        }
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn take_segments(&mut self) -> Vec<Segment> {
        mem::take(&mut self.segments)
    }

    pub fn clear_segments(&mut self) {
        self.segments.clear();
    }
}

impl Index<u32> for DynamicOctree {
    type Output = Node;

    fn index(&self, index: u32) -> &Self::Output {
        &self.octree[index]
    }
}

impl IndexMut<u32> for DynamicOctree {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        self.push_segment(Segment::new(index, 1));
        &mut self.octree[index]
    }
}

impl Index<Range<u32>> for DynamicOctree {
    type Output = [Node];

    fn index(&self, index: Range<u32>) -> &Self::Output {
        &self.octree[index]
    }
}

impl IndexMut<Range<u32>> for DynamicOctree {
    fn index_mut(&mut self, index: Range<u32>) -> &mut Self::Output {
        self.push_segment(Segment::new(index.start, index.end - index.start));
        &mut self.octree[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_join() {
        let a = Segment::new(0, 10);
        let b = Segment::new(5, 10);

        assert_eq!(a.join(b), Segment::new(0, 15));
    }

    #[test]
    fn push_segment() {
        let mut octree = DynamicOctree::empty();

        octree.push_segment(Segment::new(0, 10));
        octree.push_segment(Segment::new(20, 10));
        octree.push_segment(Segment::new(5, 10));

        assert_eq!(octree.segments, vec![Segment::new(0, 30)]);

        octree.push_segment(Segment::new(2048, 10));

        assert_eq!(
            octree.segments,
            vec![Segment::new(0, 30), Segment::new(2048, 10)]
        );
    }
}
