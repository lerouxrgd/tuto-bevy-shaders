#![allow(clippy::redundant_field_names)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::ecs::system::lifetimeless::SRes;
use bevy::ecs::system::SystemParamItem;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::camera::ScalingMode;
use bevy::render::render_asset::{PrepareAssetError, RenderAsset, RenderAssets};
use bevy::render::render_resource::std140::{AsStd140, Std140};
use bevy::render::render_resource::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferInitDescriptor,
    BufferSize, BufferUsages, SamplerBindingType, ShaderStages, TextureSampleType,
    TextureViewDimension,
};
use bevy::render::renderer::RenderDevice;
use bevy::sprite::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle};
use bevy::window::PresentMode;
use bevy_inspector_egui::{WorldInspectorParams, WorldInspectorPlugin};

pub const CLEAR: Color = Color::rgb(0.3, 0.3, 0.3);
pub const HEIGHT: f32 = 900.0;
pub const RESOLUTION: f32 = 16.0 / 9.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(CLEAR))
        .insert_resource(WindowDescriptor {
            width: HEIGHT * RESOLUTION,
            height: HEIGHT,
            title: "Bevy Template".to_string(),
            present_mode: PresentMode::Fifo,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<MyMaterial>::default())
        .add_plugin(WorldInspectorPlugin::new())
        .insert_resource(WorldInspectorParams {
            enabled: false,
            ..Default::default()
        })
        .add_startup_system_to_stage(StartupStage::PreStartup, load_image)
        .add_startup_system(spawn_camera)
        .add_startup_system(spawn_quad)
        .add_system(toggle_inspector)
        .run();
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Deref)]
pub struct Awesome(Handle<Image>);

#[derive(TypeUuid, Clone)]
#[uuid = "895efe95-d4a1-45dc-a5c3-7506a94dbc13"] // Make it impl Asset
struct MyMaterial {
    alpha: f32,
    color: Color,
    image: Handle<Image>,
}

#[derive(Clone, AsStd140)]
struct MyMaterialUniformData {
    alpha: f32,
    color: Vec4,
}

impl Material2d for MyMaterial {
    fn bind_group(
        material: &<Self as RenderAsset>::PreparedAsset,
    ) -> &bevy::render::render_resource::BindGroup {
        &material.bind_group
    }

    fn bind_group_layout(
        render_device: &bevy::render::renderer::RenderDevice,
    ) -> bevy::render::render_resource::BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            MyMaterialUniformData::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("my_material.wgsl"))
    }
}

impl RenderAsset for MyMaterial {
    type ExtractedAsset = MyMaterial;
    type PreparedAsset = MyMaterialGpu;
    type Param = (
        SRes<RenderDevice>,
        SRes<Material2dPipeline<MyMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<MyMaterial>> {
        let data = MyMaterialUniformData {
            alpha: extracted_asset.alpha,
            color: extracted_asset.color.as_linear_rgba_f32().into(),
        };

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: data.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let (view, sampler) = if let Some(result) = pipeline
            .mesh2d_pipeline
            .get_image_texture(gpu_images, &Some(extracted_asset.image.clone()))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.material2d_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });
        Ok(MyMaterialGpu { bind_group })
    }
}

struct MyMaterialGpu {
    bind_group: BindGroup,
}

////////////////////////////////////////////////////////////////////////////////////////

fn spawn_quad(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut my_material_assets: ResMut<Assets<MyMaterial>>,
    awesome_face: Res<Awesome>,
) {
    commands.spawn_bundle(MaterialMesh2dBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Quad::default())).into(),
        material: my_material_assets.add(MyMaterial {
            alpha: 0.5,
            color: Color::RED,
            image: awesome_face.clone(),
        }),
        ..default()
    });
}

fn load_image(mut commands: Commands, assets: Res<AssetServer>) {
    let awesome = assets.load("awesome.png");
    commands.insert_resource(Awesome(awesome));
}

fn spawn_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();

    camera.orthographic_projection.right = 1.0 * RESOLUTION;
    camera.orthographic_projection.left = -1.0 * RESOLUTION;

    camera.orthographic_projection.top = 1.0;
    camera.orthographic_projection.bottom = -1.0;

    camera.orthographic_projection.scaling_mode = ScalingMode::None;

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
