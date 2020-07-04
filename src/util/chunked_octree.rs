use {
  super::ZOrder,
  std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
  },
};

const START_INDEX_LOOKUP: [usize; 11] = [
  0, 1, 9, 73, 585, 4681, 37449, 299593, 2396745, 19173961, 153391689,
];

pub struct ChunkedOctree<T>
where
  T: Default + Copy,
{
  depth: u8,
  chunks: HashMap<ZOrder, Region<T>>,
}

struct Region<T>(Vec<T>);

impl<T> ChunkedOctree<T>
where
  T: Default + Copy,
{
  pub fn new(depth: u8) -> Self {
    assert!(depth < START_INDEX_LOOKUP.len() as u8 - 1);
    Self {
      depth,
      chunks: HashMap::new(),
    }
  }

  pub fn depth(&self) -> u8 {
    self.depth
  }

  pub fn find<'a, W, F>(&'a self, weight_fn: W, filter_fn: F) -> ChunkedOctreeIterator<'a, T, W, F>
  where
    W: Fn(u8, ZOrder) -> Option<f32>,
    F: Fn(&T) -> bool,
  {
    ChunkedOctreeIterator {
      octree: &self,
      weight_fn,
      filter_fn,
      checked_regions: HashSet::new(),
      processing: BinaryHeap::new(),
    }
  }

  pub fn get(&self, level: u8, node_pos: ZOrder) -> T {
    self
      .chunks
      .get(&(node_pos >> self.depth as usize))
      .map(|region| {
        let base_index = START_INDEX_LOOKUP[(self.depth - level) as usize];
        let local_index = (node_pos.raw() as usize) & !(!0 << (self.depth * 3));
        region.0[base_index + local_index]
      })
      .unwrap_or_default()
  }

  pub fn update<U, B>(&mut self, node_pos: ZOrder, update_fn: U, bubble_fn: B)
  where
    U: FnOnce(&mut T),
    B: Fn(u8, &[T], &mut T) -> bool,
  {
    let region_pos = node_pos >> self.depth as usize;
    let mut local_pos = ZOrder::from_raw(node_pos.raw() & !(!0 << (self.depth * 3)));

    let depth = self.depth; // Need to create this so it can be used in `or_insert_with` closure.
    let region = self.chunks.entry(region_pos).or_insert_with(|| {
      let size = START_INDEX_LOOKUP[depth as usize + 1] + 1;
      Region(vec![Default::default(); size])
    });

    let index = START_INDEX_LOOKUP[self.depth as usize] + local_pos.raw() as usize;
    let value = region.0.get_mut(index).unwrap();
    update_fn(value);

    for level in 1..=self.depth {
      let children_start = START_INDEX_LOOKUP[(self.depth - (level - 1)) as usize];
      let children_index = children_start + (local_pos.raw() & !0b111) as usize;

      local_pos = local_pos >> 1;
      let parent_start = START_INDEX_LOOKUP[(self.depth - level) as usize];
      let parent_index = parent_start + local_pos.raw() as usize;

      let split = region.0.split_at_mut(children_index);
      let children = &split.1[0..8];
      let parent = split.0.get_mut(parent_index).unwrap();

      if !bubble_fn(level, children, parent) {
        break;
      }
    }
  }
}

pub struct ChunkedOctreeIterator<'a, T, W, F>
where
  T: Default + Copy,
  W: Fn(u8, ZOrder) -> Option<f32>,
  F: Fn(&T) -> bool,
{
  octree: &'a ChunkedOctree<T>,
  weight_fn: W,
  filter_fn: F,
  checked_regions: HashSet<ZOrder>,
  processing: BinaryHeap<ProcessingNode>,
}

impl<'a, T, W, F> ChunkedOctreeIterator<'a, T, W, F>
where
  T: Default + Copy,
  W: Fn(u8, ZOrder) -> Option<f32>,
  F: Fn(&T) -> bool,
{
  pub fn search(mut self, node_pos: ZOrder) -> Self {
    let region_pos = node_pos >> self.octree.depth as usize;
    for x in -1..=1 {
      for y in -1..=1 {
        for z in -1..=1 {
          let offset = ZOrder::new(x, y, z).unwrap();
          self.search_region(region_pos + offset);
        }
      }
    }
    self
  }

  fn search_region(&mut self, region_pos: ZOrder) {
    if self.checked_regions.insert(region_pos) {
      self.push_node(self.octree.depth, region_pos);
    }
  }

  fn push_node(&mut self, level: u8, node_pos: ZOrder) {
    if let Some(weight) = (&self.weight_fn)(level, node_pos) {
      if (&self.filter_fn)(&self.octree.get(level, node_pos)) {
        self.processing.push(ProcessingNode {
          weight,
          level,
          node_pos,
        });
      }
    }
  }
}

impl<'a, T, W, F> Iterator for ChunkedOctreeIterator<'a, T, W, F>
where
  T: Default + Copy,
  W: Fn(u8, ZOrder) -> Option<f32>,
  F: Fn(&T) -> bool,
{
  type Item = (ZOrder, f32);

  fn next(&mut self) -> Option<Self::Item> {
    while let Some(node) = self.processing.pop() {
      if node.level == 0 {
        return Some((node.node_pos, node.weight));
      } else {
        for i in 0..8 {
          self.push_node(node.level - 1, (node.node_pos << 1) | ZOrder::from_raw(i));
        }
      }
    }
    None
  }
}

struct ProcessingNode {
  weight: f32,
  level: u8,
  node_pos: ZOrder,
}

impl Ord for ProcessingNode {
  fn cmp(&self, rhs: &Self) -> Ordering {
    // NOTE: Does not handle NaN, but that's not a valid weight anyway.
    // Since we want a min-heap, we swap `Greater` and `Less`.
    if self.weight > rhs.weight {
      Ordering::Less
    } else if self.weight < rhs.weight {
      Ordering::Greater
    } else {
      Ordering::Equal
    }
  }
}
impl PartialOrd for ProcessingNode {
  fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
    Some(self.cmp(rhs))
  }
}

impl Eq for ProcessingNode {}
impl PartialEq for ProcessingNode {
  fn eq(&self, rhs: &Self) -> bool {
    // NOTE: Does not handle NaN!
    self.weight == rhs.weight
  }
}
