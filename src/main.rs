use {
  crate::bloxel::{
    chunk::{storage::*, *},
    ChunkMeshGenerator,
  },
  amethyst::{
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
      plugins::{RenderShaded3D, RenderToWindow},
      rendy::mesh::{Normal, Position, TexCoord},
      types::DefaultBackend,
      RenderingBundle,
    },
    utils::{
      application_root_dir,
      auto_fov::{AutoFov, AutoFovSystem},
    },
    winit::{MouseButton, VirtualKeyCode},
    Error,
  },
  log::{error, info},
  rand::prelude::*,
  serde::{Deserialize, Serialize},
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
    .with(ChunkMeshGenerator::default(), "", &[])
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
