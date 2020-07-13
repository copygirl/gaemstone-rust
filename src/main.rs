#[macro_use]
extern crate bitflags;

use {
  crate::{
    bloxel::{
      chunk::{ChunkLookupSystemDesc, ChunkState},
      ChunkMeshGenerator, WorldGenerator,
    },
    util::ChunkedOctree,
  },
  amethyst::{
    assets::*,
    controls::{ControlTagPrefab, FlyControlBundle, HideCursor},
    core::transform::{Transform, TransformBundle},
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
    // ====================
    // == Loading Assets ==
    // ====================
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
    // ==============
    // == Controls ==
    // ==============
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
      .with_speed(20.0),
    )?
    // ===========================
    // == World / Chunk related ==
    // ===========================
    .with_system_desc(ChunkLookupSystemDesc::default(), "chunk_lookup", &[])
    .with(WorldGenerator::default(), "world_gen", &["chunk_lookup"])
    .with(
      ChunkMeshGenerator::default(),
      "chunk_mesh_gen",
      &["chunk_lookup"],
    )
    // =======================
    // == Rendering related ==
    // =======================
    .with(AutoFovSystem::new(), "auto_fov", &[])
    .with_bundle(TransformBundle::new().with_dep(&["fly_movement", "free_rotation"]))?
    .with_bundle(
      RenderingBundle::<DefaultBackend>::new()
        .with_plugin(RenderToWindow::from_config_path(config_path_display)?.with_clear(CLEAR_COLOR))
        .with_plugin(RenderShaded3D::default()),
    )?;

  let mut game = Application::build(assets_dir, MainState::default())?.build(game_data)?;
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
struct MainState;

impl SimpleState for MainState {
  fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
    data.world.insert(ChunkedOctree::<ChunkState>::new(5));
    let handle = data.world.exec(|loader: PrefabLoader<ScenePrefab>| {
      loader.load("prefab/basic_scene.ron", RonFormat, ())
    });
    data.world.create_entity().with(handle).build();
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
