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
        .add_systems(Update, update_material)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
    window: Query<&Window>,
) {
    let win = window.single().unwrap();

    commands.spawn((Camera2d::default(),));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(MandelbulbMaterial {
            resolution: Vec2::new(win.width(), win.height()),

            time: 0.0,
            power: 8.0,
            speed: 0.2,

            ray_steps: 100,   // How far to march (higher = less artifacts at edges)
            mandel_iters: 20, // Fractal detail (higher = more spikes)
            max_dist: 40.0,   // Render distance clipping
            hit_threshold: 0.002, // Precision (lower = sharper but slower)
            camera_zoom: 2.5,
        })),
        Transform::default().with_scale(Vec3::splat(1280.0)),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct MandelbulbMaterial {
    #[uniform(0)]
    resolution: Vec2, // 8 bytes (Aligned)
    #[uniform(0)]
    time: f32, // 4 bytes
    #[uniform(0)]
    power: f32, // 4 bytes
    #[uniform(0)]
    speed: f32, // 4 bytes
    #[uniform(0)]
    ray_steps: u32, // 4 bytes (Integers are fine in Uniforms)
    #[uniform(0)]
    mandel_iters: u32, // 4 bytes
    #[uniform(0)]
    max_dist: f32, // 4 bytes
    #[uniform(0)]
    hit_threshold: f32, // 4 bytes
    #[uniform(0)]
    camera_zoom: f32, // 4 bytes
}

impl Material2d for MandelbulbMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/mandelbulb.wgsl".into()
    }
}

// System to update the time uniform every frame
fn update_material(
    time: Res<Time>,
    window: Query<&Window>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) {
    let win = window.single().unwrap();
    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_secs_f64() as f32;
        material.resolution = Vec2::new(win.width(), win.height());

        // Animate the power parameter over time
        // material.power = 8.0 + (time.elapsed_secs_f64().sin() as f32);
        material.camera_zoom = 2.75 + (time.elapsed_secs_f64().sin() as f32) * 0.25;
    }
}
