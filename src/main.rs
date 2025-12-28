use bevy::sprite_render::{Material2d, Material2dPlugin};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            Material2dPlugin::<MandelbulbMaterial>::default(),
        ))
        .init_resource::<SimSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, update_material)
        .add_systems(EguiPrimaryContextPass, ui_controls)
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

    let material_handle = materials.add(MandelbulbMaterial {
        resolution: Vec2::new(win.width(), win.height()),
        time: 0.0,
        power: 8.0,
        speed: 0.2,
        ray_steps: 100,
        mandel_iters: 20,
        max_dist: 40.0,
        hit_threshold: 0.002,
        camera_zoom: 2.5,
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(material_handle),
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
    settings: Res<SimSettings>,
) {
    let win = window.single().unwrap();
    for (_, material) in materials.iter_mut() {
        material.time = time.elapsed_secs_f64() as f32;
        material.resolution = Vec2::new(win.width(), win.height());

        // Animate the power parameter over time
        // material.power = 8.0 + (time.elapsed_secs_f64().sin() as f32);
        if settings.animate_zoom {
            material.camera_zoom = 2.75 + (time.elapsed_secs_f64().sin() as f32) * 0.25;
        }
    }
}

#[derive(Resource)]
struct SimSettings {
    animate_zoom: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            animate_zoom: false,
        }
    }
}

fn ui_controls(
    mut contexts: EguiContexts,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
    mut settings: ResMut<SimSettings>,
) {
    let ctx = contexts.ctx_mut().unwrap();

    egui::Window::new("Mandelbulb Settings")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Fractal Parameters");

            // We need to iterate over materials (even though there is only one)
            for (_, mat) in materials.iter_mut() {
                ui.separator();
                ui.label("Shape");
                ui.add(egui::Slider::new(&mut mat.power, 1.0..=16.0).text("Power"));

                let mut iters = mat.mandel_iters as f32;
                if ui
                    .add(egui::Slider::new(&mut iters, 1.0..=50.0).text("Iterations"))
                    .changed()
                {
                    mat.mandel_iters = iters as u32;
                }

                ui.separator();
                ui.label("Rendering Quality");
                let mut steps = mat.ray_steps as f32;
                if ui
                    .add(egui::Slider::new(&mut steps, 10.0..=300.0).text("Ray Steps"))
                    .changed()
                {
                    mat.ray_steps = steps as u32;
                }
                ui.add(
                    egui::Slider::new(&mut mat.hit_threshold, 0.0001..=0.01)
                        .text("Threshold")
                        .logarithmic(true),
                );
                ui.add(egui::Slider::new(&mut mat.max_dist, 10.0..=100.0).text("Max Dist"));

                ui.separator();
                ui.label("Camera & Animation");
                ui.add(egui::Slider::new(&mut mat.camera_zoom, 0.1..=10.0).text("Zoom"));
                ui.add(egui::Slider::new(&mut mat.speed, 0.0..=2.0).text("Rotation Speed"));

                ui.separator();
                ui.checkbox(&mut settings.animate_zoom, "Auto-Animate Zoom");
            }
        });
}
