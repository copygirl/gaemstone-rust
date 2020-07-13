use {
  crate::{
    bloxel::{
      chunk::{storage::*, *},
      Facing,
    },
    util::{ChunkedOctree, ZOrder},
  },
  amethyst::{
    assets::*,
    ecs::prelude::*,
    renderer::{
      loaders::load_from_srgb,
      palette::rgb::Srgb,
      rendy::mesh::{MeshBuilder, Normal, Position, TexCoord},
      types::{Mesh, TextureData},
      Material, MaterialDefaults, Texture,
    },
  },
};

#[derive(Default)]
pub struct ChunkMeshGenerator;

// TODO: Use lazy static for the material?
pub struct WhiteMaterial(Handle<Material>);

impl<'a> System<'a> for ChunkMeshGenerator {
  type SystemData = (
    Entities<'a>,
    Read<'a, LazyUpdate>,
    ReadExpect<'a, Loader>,
    ReadExpect<'a, MaterialDefaults>,
    ReadExpect<'a, AssetStorage<Texture>>,
    ReadExpect<'a, AssetStorage<Material>>,
    ReadExpect<'a, AssetStorage<Mesh>>,
    Read<'a, ChunkLookup>,
    ReadStorage<'a, ChunkStorage<u8>>,
    Write<'a, Option<WhiteMaterial>>,
    WriteExpect<'a, ChunkedOctree<ChunkState>>,
  );

  fn run(
    &mut self,
    (
      entities,
      lazy,
      loader,
      material_defaults,
      texture_storage,
      material_storage,
      mesh_storage,
      chunk_lookup,
      chunk_storages,
      mut gen_resources,
      mut octree,
    ): Self::SystemData,
  ) {
    let res = gen_resources.get_or_insert_with(|| {
      let white_texture = loader.load_from_data(
        TextureData(load_from_srgb(Srgb::new(1., 1., 1.))),
        (),
        &texture_storage,
      );
      let white_material = loader.load_from_data(
        Material {
          albedo: white_texture.clone(),
          ..material_defaults.0.clone()
        },
        (),
        &material_storage,
      );
      WhiteMaterial(white_material)
    });

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
        |state| (*state & ChunkState::MESH_UPDATED_ALL) != ChunkState::MESH_UPDATED_ALL,
      )
      .search(ZOrder::new(0, 0, 0).unwrap())
      .take(4)
      .filter_map(|(z_pos, _)| {
        let (x, y, z) = z_pos.into();
        let chunk_pos = ChunkPos::new(x, y, z);
        let entity = chunk_lookup.get(chunk_pos);
        entity.map(|e| (chunk_pos, z_pos, e))
      })
      .collect::<Vec<_>>();

    // TODO: ChunkPos should use ZOrder and thus make it unnecessary to keep them both or convert between them.
    for (_chunk_pos, z_pos, entity) in nearest {
      if let Some(storage) = chunk_storages.get(entity) {
        let mut indices = vec![];
        let mut pos = vec![];
        let mut norm = vec![];
        let mut tex = vec![];

        static TRIANGLE_INDICES: [u16; 6] = [0, 1, 3, 1, 2, 3];
        static OFFSETS_PER_FACING: [[[i32; 3]; 4]; 6] = [
          [[1, 1, 1], [1, 0, 1], [1, 0, 0], [1, 1, 0]], // +X
          [[0, 1, 0], [0, 0, 0], [0, 0, 1], [0, 1, 1]], // -X
          [[1, 1, 0], [0, 1, 0], [0, 1, 1], [1, 1, 1]], // +Y
          [[1, 0, 1], [0, 0, 1], [0, 0, 0], [1, 0, 0]], // -Y
          [[0, 1, 1], [0, 0, 1], [1, 0, 1], [1, 1, 1]], // +Z
          [[1, 1, 0], [1, 0, 0], [0, 0, 0], [0, 1, 0]], // +Z
        ];

        for x in 0..CHUNK_LENGTH as i32 {
          for y in 0..CHUNK_LENGTH as i32 {
            for z in 0..CHUNK_LENGTH as i32 {
              // SAFETY: Bounds should be safe due to loop only going over valid values.
              let index = unsafe { Index::new_unchecked(x, y, z) };
              if storage.get(index) == 0 {
                continue;
              }
              for face in Facing::iter_all() {
                let (fx, fy, fz) = face.into();

                // Skip drawing this face if there's another block in that direction.
                // `get` returns `ChunkBoundsError` if coords are outside of the bounds of
                // the storage, so we can make use of that to avoid checking this ourselves.
                if Index::new(x + fx, y + fy, z + fz)
                  .map(|index| storage.get(index))
                  .unwrap_or_default()
                  > 0
                {
                  continue;
                }

                for i in &TRIANGLE_INDICES {
                  indices.push(pos.len() as u16 + i);
                }
                let offsets = OFFSETS_PER_FACING[match face {
                  Facing::East => 0,
                  Facing::West => 1,
                  Facing::Up => 2,
                  Facing::Down => 3,
                  Facing::South => 4,
                  Facing::North => 5,
                }];
                for i in 0..4 {
                  let offset = offsets[i];
                  pos.push(Position([
                    (x + offset[0]) as f32,
                    (y + offset[1]) as f32,
                    (z + offset[2]) as f32,
                  ]));
                  norm.push(Normal([fx as f32, fy as f32, fz as f32]));
                  tex.push(TexCoord(match i {
                    0 => [0.0, 0.0],
                    1 => [0.0, 1.0],
                    2 => [1.0, 1.0],
                    3 => [1.0, 0.0],
                    _ => panic!(),
                  }));
                }
              }
            }
          }
        }

        if indices.is_empty() {
          // FIXME: This is a temporary solution.
          entities.delete(entity).unwrap();
        } else {
          let mesh_builder = MeshBuilder::new()
            .with_indices(indices)
            .with_vertices(pos)
            .with_vertices(norm)
            .with_vertices(tex)
            .into_owned();
          let mesh = loader.load_from_data(mesh_builder.into(), (), &mesh_storage);

          lazy.insert(entity, mesh);
          lazy.insert(entity, res.0.clone());
        }

        const MASK_SOME: ChunkState = ChunkState::MESH_UPDATED_SOME;
        const MASK_ALL: ChunkState = ChunkState::MESH_UPDATED_ALL;
        octree.update(
          z_pos,
          |state| *state = *state | MASK_ALL,
          |_level, children, parent| {
            let mask = if children.iter().all(|s| *s & MASK_ALL == MASK_ALL) {
              MASK_ALL
            } else {
              MASK_SOME
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
}
