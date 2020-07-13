use {
  super::chunk::{storage::*, *},
  crate::util::{ChunkedOctree, ZOrder},
  amethyst::{
    core::{math::Vector3, transform::Transform},
    ecs::prelude::*,
    renderer::visibility::BoundingSphere,
  },
  noise::{NoiseFn, OpenSimplex},
};

#[derive(Default)]
pub struct WorldGenerator;

impl<'a> System<'a> for WorldGenerator {
  type SystemData = (
    Entities<'a>,
    ReadExpect<'a, LazyUpdate>,
    WriteExpect<'a, ChunkedOctree<ChunkState>>,
  );

  fn run(&mut self, (entities, lazy, mut octree): Self::SystemData) {
    const MAX_DISTANCE_SQUARED: f32 = 8.5 * 8.5;
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
        |state| (*state & ChunkState::GENERATED_ALL) != ChunkState::GENERATED_ALL,
      )
      .search(ZOrder::new(0, 0, 0).unwrap())
      .take(4)
      .collect::<Vec<_>>();

    for (pos, _) in nearest {
      let state = octree.get(0, pos);
      // TODO: This should handle chunk entities which already exist, rather than creating them manually.

      let (x, y, z) = pos.into();
      let chunk_pos = ChunkPos::new(x as i32, y as i32, z as i32);
      let position = Vector3::new(
        (x << CHUNK_LENGTH_BITS as i64) as f32,
        (y << CHUNK_LENGTH_BITS as i64) as f32,
        (z << CHUNK_LENGTH_BITS as i64) as f32,
      );

      let noise = OpenSimplex::new();
      let mut storage = PaletteStorageImpl::<u8>::new();
      for x in 0..CHUNK_LENGTH as i32 {
        for y in 0..CHUNK_LENGTH as i32 {
          for z in 0..CHUNK_LENGTH as i32 {
            let fx = (position.x as f64 + x as f64 + 0.5) / 16.0;
            let fy = (position.y as f64 + y as f64 + 0.5) / 16.0;
            let fz = (position.z as f64 + z as f64 + 0.5) / 16.0;
            let bias = (fy / 4.0).max(0.0).min(2.0);
            if noise.get([fx, fy, fz]) > bias {
              // SAFETY: Bounds should be safe due to loop only going over valid values.
              let index = unsafe { Index::new_unchecked(x, y, z) };
              storage.set(index, 1u8);
            }
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

      let mask_some = ChunkState::EXISTS_SOME | ChunkState::GENERATED_SOME;
      let mask_all = ChunkState::EXISTS_ALL | ChunkState::GENERATED_ALL;
      octree.update(
        pos,
        |state| *state = *state | mask_all,
        |_level, children, parent| {
          let mask = if children.iter().all(|s| *s & mask_all == mask_all) {
            mask_all
          } else {
            mask_some
          };
          if *parent & mask == mask {
            false
          } else {
            *parent = *parent | mask;
            true
          }
        },
      );
    }
  }
}
