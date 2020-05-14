use amethyst::{
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
use rand;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

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
    .with(VoxelMeshGenerator::<DefaultBackend>::default(), "", &[])
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

struct VoxelMeshGenerator<B> {
  _backend: PhantomData<B>,
  created_entity: bool,
}

impl<B> Default for VoxelMeshGenerator<B> {
  fn default() -> Self {
    VoxelMeshGenerator {
      _backend: PhantomData,
      created_entity: false,
    }
  }
}

impl<'a, B: Backend> System<'a> for VoxelMeshGenerator<B> {
  type SystemData = (
    Entities<'a>,
    ReadExpect<'a, Loader>,
    ReadExpect<'a, MaterialDefaults>,
    ReadExpect<'a, AssetStorage<Texture>>,
    ReadExpect<'a, AssetStorage<Material>>,
    ReadExpect<'a, AssetStorage<Mesh>>,
    WriteStorage<'a, Transform>,
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
      mut transforms,
      mut meshes,
      mut materials,
    ): Self::SystemData,
  ) {
    if self.created_entity {
      return;
    }

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

    let mut indices = vec![];
    let mut pos = vec![];
    let mut norm = vec![];
    let mut tex = vec![];

    let triangle_indices = [0, 1, 3, 1, 2, 3];
    let normals_per_facing = [
      Normal([1.0, 0.0, 0.0]),  // +X
      Normal([-1.0, 0.0, 0.0]), // -X
      Normal([0.0, 1.0, 0.0]),  // +Y
      Normal([0.0, -1.0, 0.0]), // -Y
      Normal([0.0, 0.0, 1.0]),  // +Z
      Normal([0.0, 0.0, -1.0]), // -Z
    ];
    let offsets_per_facing = [
      [[1, 1, 1], [1, 0, 1], [1, 0, 0], [1, 1, 0]], // +X
      [[0, 1, 0], [0, 0, 0], [0, 0, 1], [0, 1, 1]], // -X
      [[1, 1, 0], [0, 1, 0], [0, 1, 1], [1, 1, 1]], // +Y
      [[1, 0, 1], [0, 0, 1], [0, 0, 0], [1, 0, 0]], // -Y
      [[0, 1, 1], [0, 0, 1], [1, 0, 1], [1, 1, 1]], // +Z
      [[1, 1, 0], [1, 0, 0], [0, 0, 0], [0, 1, 0]], // +Z
    ];

    for x in 0..16 {
      for y in 0..16 {
        for z in 0..16 {
          if rand::random() {
            continue;
          }
          for face in 0..6 {
            for i in &triangle_indices {
              indices.push(pos.len() as u16 + i);
            }
            for i in 0..4 {
              let offset = offsets_per_facing[face][i];
              pos.push(Position([
                (x + offset[0]) as f32,
                (y + offset[1]) as f32,
                (z + offset[2]) as f32,
              ]));
              norm.push(normals_per_facing[face]);
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

    let mesh = MeshBuilder::new()
      .with_indices(indices)
      .with_vertices(pos)
      .with_vertices(norm)
      .with_vertices(tex)
      .into_owned();
    let mesh = loader.load_from_data(mesh.into(), (), &mesh_storage);

    entities
      .build_entity()
      .with(Default::default(), &mut transforms)
      .with(mesh, &mut meshes)
      .with(white_material, &mut materials)
      .build();

    self.created_entity = true;
  }
}
