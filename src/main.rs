use bevy::sprite_render::{Material2d, Material2dPlugin};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<MandelbulbMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update_material_time)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) {
    commands.spawn((
        Camera2d::default(),
        // Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(MandelbulbMaterial {time: 0.0})),
        Transform::default().with_scale(Vec3::splat(1280.0)),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct MandelbulbMaterial {
    #[uniform(0)]
    time: f32,
}

impl Material2d for MandelbulbMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/mandelbulb.wgsl".into()
    }
}

// System to update the time uniform every frame
fn update_material_time(time: Res<Time>, mut materials: ResMut<Assets<MandelbulbMaterial>>) {
    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_secs_f64() as f32;
    }
}
