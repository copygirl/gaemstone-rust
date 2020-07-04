use {
  super::chunk::{storage::*, *},
  crate::util::{ChunkedOctree, ZOrder},
  amethyst::{
    core::{math::Vector3, transform::Transform},
    ecs::prelude::*,
    renderer::visibility::BoundingSphere,
  },
  rand::prelude::*,
};

bitflags! {
  #[derive(Default)]
  pub struct ChunkState: u8 {
    const EXISTS = 0b00000001;
    const GENERATED = 0b00000010;
    const UPDATED = 0b00000100;
  }
}

#[derive(Default)]
pub struct WorldGenerator;

impl<'a> System<'a> for WorldGenerator {
  type SystemData = (
    Entities<'a>,
    ReadExpect<'a, LazyUpdate>,
    WriteExpect<'a, ChunkedOctree<ChunkState>>,
  );

  fn run(&mut self, (entities, lazy, mut octree): Self::SystemData) {
    const MAX_DISTANCE_SQUARED: f32 = 6.5 * 6.5;

    let nearest = octree
      .find(
        |level, pos| {
          let (mut x, mut y, mut z) = (pos << level as usize).into();
          if x < 0 {
            x += 1 << level;
          }
          if y < 0 {
            y += 1 << level;
          }
          if z < 0 {
            z += 1 << level;
          }

          let (x, y, z) = (x as f32, y as f32, z as f32);
          let distance = x * x + y * y + z * z;
          if distance <= MAX_DISTANCE_SQUARED {
            Some(distance)
          } else {
            None
          }
        },
        |state| (*state & ChunkState::GENERATED) == Default::default(),
      )
      .search(ZOrder::new(0, 0, 0).unwrap())
      .take(1)
      .collect::<Vec<_>>();

    for (pos, _) in nearest {
      let state = octree.get(0, pos);
      // TODO: This should handle chunk entities which already exist.
      assert_ne!(state & ChunkState::EXISTS, ChunkState::EXISTS);

      let (x, y, z) = pos.into();
      let chunk_pos = ChunkPos::new(x as i32, y as i32, z as i32);
      let position = Vector3::new(
        (x << CHUNK_LENGTH_BITS as i64) as f32,
        (y << CHUNK_LENGTH_BITS as i64) as f32,
        (z << CHUNK_LENGTH_BITS as i64) as f32,
      );

      let mut rng = thread_rng();
      let mut storage = PaletteStorageImpl::<u8>::new();
      // TODO: Replace nested loop with single iterator that provides `(x, y, z, index)`?
      for x in 0..CHUNK_LENGTH as i32 {
        for y in 0..CHUNK_LENGTH as i32 {
          for z in 0..CHUNK_LENGTH as i32 {
            // SAFETY: Bounds should be safe due to loop only going over valid values.
            let index = unsafe { Index::new_unchecked(x, y, z) };
            storage.set(index, rng.gen_range(0, 2));
          }
        }
      }

      const HALF_CHUNK_LENGTH: i64 = 1 << (CHUNK_LENGTH_BITS - 1);
      const CENTER: [f32; 3] = [HALF_CHUNK_LENGTH as f32; 3];
      const RADIUS: f32 = (HALF_CHUNK_LENGTH * HALF_CHUNK_LENGTH * 3) as f32;

      lazy
        .create_entity(&entities)
        .with(Chunk { pos: chunk_pos })
        .with(ChunkStorage::new(storage))
        .with(Transform::from(position))
        .with(BoundingSphere::new(CENTER.into(), RADIUS.sqrt()))
        .build();

      let mask = ChunkState::EXISTS | ChunkState::GENERATED;
      octree.update(
        pos,
        |state| *state = *state | mask,
        |_level, children, parent| {
          if children.iter().all(|s| *s & mask == mask) {
            *parent = *parent | mask;
            true
          } else {
            false
          }
        },
      );
    }
  }
}
