use {
  crate::bloxel::{
    chunk::{storage::*, *},
    Facing,
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
    ReadStorage<'a, Chunk>,
    ReadStorage<'a, ChunkStorage<u8>>,
    ReadStorage<'a, Handle<Mesh>>,
    Write<'a, Option<WhiteMaterial>>,
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
      chunks,
      chunk_storages,
      meshes,
      mut gen_resources,
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

    for (entity, _, storage, _) in (&entities, &chunks, &chunk_storages, !&meshes)
      .join()
      .take(1)
    {
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
  }
}
