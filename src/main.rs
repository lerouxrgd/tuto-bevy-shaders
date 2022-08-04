#![allow(clippy::redundant_field_names)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::camera::ScalingMode;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::{
    encase, AsBindGroup, OwnedBindingResource, ShaderRef, ShaderType,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::{Extract, RenderApp, RenderStage};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle, RenderMaterials2d};
use bevy::window::PresentMode;
use bevy_inspector_egui::{
    Inspectable, RegisterInspectable, WorldInspectorParams, WorldInspectorPlugin,
};

pub const CLEAR: Color = Color::rgb(0.3, 0.3, 0.3);
pub const HEIGHT: f32 = 900.0;
pub const RESOLUTION: f32 = 16.0 / 9.0;

fn main() {
    let mut app = App::new();

    // Add all main world systems/resources
    app.insert_resource(ClearColor(CLEAR))
        .insert_resource(WindowDescriptor {
            width: HEIGHT * RESOLUTION,
            height: HEIGHT,
            title: "Bevy Material Tutorial".to_string(),
            present_mode: PresentMode::Fifo,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<CoolMaterial>::default())
        .add_plugin(ExtractResourcePlugin::<ExtractedTime>::default())
        .add_plugin(WorldInspectorPlugin::new())
        .insert_resource(WorldInspectorParams {
            enabled: false,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_startup_system(spawn_camera)
        .add_system(toggle_inspector)
        .register_inspectable::<Health>();

    // Add all render world systems/resources
    app.sub_app_mut(RenderApp)
        .add_system_to_stage(RenderStage::Extract, extract_health)
        .add_system_to_stage(RenderStage::Prepare, prepare_my_material);

    app.run();
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "895efe95-d4a1-45dc-a5c3-7506a94dbc13"] // Make it impl Asset
pub struct CoolMaterial {
    #[uniform(0)]
    color: Color,
    #[uniform(0)]
    time: f32,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
}

impl Material2d for CoolMaterial {
    fn fragment_shader() -> ShaderRef {
        "my_material.wgsl".into()
    }
}

#[derive(Clone, ShaderType)]
struct CoolMaterialUniformData {
    color: Color,
    time: f32,
}

#[derive(Component, Clone, Copy, Inspectable)]
struct Health {
    #[inspectable(min = 0.0, max = 1.0)]
    value: f32,
}

struct ExtractedTime {
    seconds_since_startup: f32,
}

impl ExtractResource for ExtractedTime {
    type Source = Time;

    fn extract_resource(time: &Self::Source) -> Self {
        ExtractedTime {
            seconds_since_startup: time.seconds_since_startup() as f32,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////

fn setup(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut my_material_assets: ResMut<Assets<CoolMaterial>>,
    assets: Res<AssetServer>,
) {
    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: mesh_assets.add(Mesh::from(shape::Quad::default())).into(),
            material: my_material_assets.add(CoolMaterial {
                color: Color::rgb(0.0, 1.0, 0.3),
                time: 0.0,
                image: assets.load("awesome.png"),
            }),
            transform: Transform::from_xyz(-0.6, 0.0, 0.0),
            ..default()
        })
        .insert(Health { value: 0.2 });

    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: mesh_assets.add(Mesh::from(shape::Quad::default())).into(),
            material: my_material_assets.add(CoolMaterial {
                color: Color::rgb(0.0, 1.0, 0.3),
                time: 0.0,
                image: assets.load("awesome.png"),
            }),
            transform: Transform::from_xyz(0.6, 0.0, 0.0),
            ..default()
        })
        .insert(Health { value: 0.8 });
}

fn extract_health(
    mut commands: Commands,
    health_query: Extract<Query<(Entity, &Health, &Handle<CoolMaterial>)>>,
) {
    for (entity, health, handle) in health_query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(*health)
            .insert(handle.clone());
    }
}

fn prepare_my_material(
    materials: Res<RenderMaterials2d<CoolMaterial>>,
    health_query: Query<(&Health, &Handle<CoolMaterial>)>,
    time: Res<ExtractedTime>,
    render_queue: Res<RenderQueue>,
) {
    for (health, handle) in health_query.iter() {
        if let Some(material) = materials.get(handle) {
            let binding = &material.bindings[2];
            if let OwnedBindingResource::Buffer(cur_buffer) = binding {
                let mut buffer = encase::UniformBuffer::new(Vec::new());
                buffer
                    .write(&CoolMaterialUniformData {
                        color: Color::rgb(health.value, 0.0, 0.0),
                        time: time.seconds_since_startup % 1.0,
                    })
                    .unwrap();
                render_queue.write_buffer(cur_buffer, 0, buffer.as_ref());
            }
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();

    camera.projection.right = 1.0 * RESOLUTION;
    camera.projection.left = -1.0 * RESOLUTION;

    camera.projection.top = 1.0;
    camera.projection.bottom = -1.0;

    camera.projection.scaling_mode = ScalingMode::None;

    commands.spawn_bundle(camera);
}

fn toggle_inspector(
    input: ResMut<Input<KeyCode>>,
    mut window_params: ResMut<WorldInspectorParams>,
) {
    if input.just_pressed(KeyCode::Grave) {
        window_params.enabled = !window_params.enabled
    }
}
