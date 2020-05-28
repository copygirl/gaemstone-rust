use amethyst::{
  assets::*,
  controls::{ControlTagPrefab, FlyControlBundle, HideCursor},
  core::{
    math::Vector3,
    transform::{Transform, TransformBundle},
  },
  derive::PrefabData,
  ecs::prelude::*,
  gltf::{GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc},
  input::{is_key_down, is_mouse_button_down, InputBundle, StringBindings},
  prelude::*,
  renderer::{
    camera::CameraPrefab,
    formats::GraphicsPrefab,
    light::LightPrefab,
    loaders::load_from_srgb,
    palette::rgb::Srgb,
    plugins::{RenderShaded3D, RenderToWindow},
    rendy::mesh::{MeshBuilder, Normal, Position, TexCoord},
    types::{Backend, DefaultBackend, Mesh, TextureData},
    Material, MaterialDefaults, RenderingBundle, Texture,
  },
  utils::{
    application_root_dir,
    auto_fov::{AutoFov, AutoFovSystem},
  },
  winit::{MouseButton, VirtualKeyCode},
  Error,
};
use log::{error, info};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::bloxel::{
  chunk::{storage::*, *},
  Facing,
};

mod bloxel;
mod util;

const CLEAR_COLOR: [f32; 4] = [0.1, 0.0, 0.3, 1.0];

fn main() -> Result<(), Error> {
  amethyst::start_logger(Default::default());

  let app_root = application_root_dir()?;
  let assets_dir = app_root.join("assets");
  let config_dir = app_root.join("config");

  let config_path_display = config_dir.join("display.ron");
  let config_path_bindings = config_dir.join("bindings.ron");

  let game_data = GameDataBuilder::default()
    .with_system_desc(
      PrefabLoaderSystemDesc::<ScenePrefab>::default(),
      "scene_loader",
      &[],
    )
    .with_system_desc(
      GltfSceneLoaderSystemDesc::default(),
      "gltf_loader",
      &["scene_loader"],
    )
    .with(AutoFovSystem::new(), "auto_fov", &["scene_loader"])
    .with(BloxelMeshGenerator::<DefaultBackend>::default(), "", &[])
    .with_bundle(
      InputBundle::<StringBindings>::new().with_bindings_from_file(&config_path_bindings)?,
    )?
    .with_bundle(
      FlyControlBundle::<StringBindings>::new(
        Some(String::from("move_x")),
        Some(String::from("move_y")),
        Some(String::from("move_z")),
      )
      .with_sensitivity(0.1, 0.1)
      .with_speed(2.0),
    )?
    .with_bundle(TransformBundle::new().with_dep(&["fly_movement", "free_rotation"]))?
    .with_bundle(
      RenderingBundle::<DefaultBackend>::new()
        .with_plugin(RenderToWindow::from_config_path(config_path_display)?.with_clear(CLEAR_COLOR))
        .with_plugin(RenderShaded3D::default()),
    )?;

  let mut game = Application::build(assets_dir, LoadingState::default())?.build(game_data)?;
  game.run();
  Ok(())
}

#[derive(Default, Deserialize, PrefabData, Serialize)]
#[serde(default)]
struct ScenePrefab {
  graphics: Option<GraphicsPrefab<(Vec<Position>, Vec<Normal>, Vec<TexCoord>)>>,
  gltf: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
  transform: Option<Transform>,
  light: Option<LightPrefab>,
  camera: Option<CameraPrefab>,
  control_tag: Option<ControlTagPrefab>,
  auto_fov: Option<AutoFov>,
}

#[derive(Default)]
struct LoadingState {
  progress: ProgressCounter,
  scene: Option<Handle<Prefab<ScenePrefab>>>,
}

impl SimpleState for LoadingState {
  fn on_start(&mut self, data: StateData<GameData>) {
    let handle = data.world.exec(|loader: PrefabLoader<'_, ScenePrefab>| {
      loader.load("prefab/basic_scene.ron", RonFormat, &mut self.progress)
    });
    self.scene = Some(handle);
  }

  fn update(&mut self, _: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
    match self.progress.complete() {
      Completion::Loading => Trans::None,
      Completion::Failed => {
        error!("Loading scene failed");
        Trans::Quit
      }
      Completion::Complete => {
        info!("Loading finished - moving to MainState");
        Trans::Switch(Box::new(MainState {
          scene: self.scene.take().unwrap(),
        }))
      }
    }
  }
}

struct MainState {
  scene: Handle<Prefab<ScenePrefab>>,
}

impl SimpleState for MainState {
  fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
    data.world.create_entity().with(self.scene.clone()).build();

    let mut rng = thread_rng();
    const CHUNK_SIZE: u32 = 16;
    for x in -8..8 {
      for z in -8..8 {
        let chunk = Chunk {
          pos: ChunkPos::new(x, -1, z),
        };
        let position = Vector3::new(
          (chunk.pos.x * CHUNK_SIZE as i32) as f32,
          (chunk.pos.y * CHUNK_SIZE as i32) as f32,
          (chunk.pos.z * CHUNK_SIZE as i32) as f32,
        );
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
        data
          .world
          .create_entity()
          .with(chunk)
          .with(Transform::from(position))
          .with(ChunkStorage::new(storage))
          .build();
      }
    }
  }

  fn handle_event(
    &mut self,
    data: StateData<'_, GameData<'_, '_>>,
    event: StateEvent,
  ) -> SimpleTrans {
    let StateData { world, .. } = data;
    if let StateEvent::Window(event) = &event {
      if is_key_down(&event, VirtualKeyCode::Escape) {
        world.write_resource::<HideCursor>().hide = false;
      } else if is_mouse_button_down(&event, MouseButton::Left) {
        world.write_resource::<HideCursor>().hide = true;
      }
    }
    Trans::None
  }
}

struct BloxelMeshGenerator<B> {
  _backend: PhantomData<B>,
}

impl<B> Default for BloxelMeshGenerator<B> {
  fn default() -> Self {
    BloxelMeshGenerator {
      _backend: PhantomData,
    }
  }
}

struct WhiteMaterial(Handle<Material>);

impl<'a, B: Backend> System<'a> for BloxelMeshGenerator<B> {
  type SystemData = (
    Entities<'a>,
    ReadExpect<'a, Loader>,
    ReadExpect<'a, MaterialDefaults>,
    ReadExpect<'a, AssetStorage<Texture>>,
    ReadExpect<'a, AssetStorage<Material>>,
    ReadExpect<'a, AssetStorage<Mesh>>,
    ReadStorage<'a, Chunk>,
    ReadStorage<'a, ChunkStorage<u8>>,
    Write<'a, Option<WhiteMaterial>>,
    WriteStorage<'a, Handle<Mesh>>,
    WriteStorage<'a, Handle<Material>>,
  );

  fn run(
    &mut self,
    (
      entities,
      loader,
      material_defaults,
      texture_storage,
      material_storage,
      mesh_storage,
      chunks,
      chunk_storages,
      mut gen_resources,
      mut meshes,
      mut materials,
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

    let mut processed_chunks = Vec::<(Entity, Handle<Mesh>)>::new();
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

      processed_chunks.push((entity, mesh));
    }
    for (entity, mesh) in processed_chunks {
      meshes.insert(entity, mesh).unwrap();
      materials.insert(entity, res.0.clone()).unwrap();
    }
  }
}
