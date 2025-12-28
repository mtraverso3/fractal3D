use bevy::input::mouse::MouseMotion;
use bevy::sprite_render::{Material2d, Material2dPlugin};
use bevy::winit::{UpdateMode, WinitSettings};
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            Material2dPlugin::<MandelbulbMaterial>::default(),
        ))
        .init_resource::<SimSettings>()
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_material, mouse_controls, manage_rendering_mode),
        )
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
        power: 8.0,
        ray_steps: 100,
        mandel_iters: 20,
        max_dist: 40.0,
        hit_threshold: 0.002,
        camera_zoom: 2.5,
        palette_id: 0,
        light_pos_x: 2.0,
        light_pos_y: 4.0,
        background_glow_intensity: 0.0,
        color_scale: 1.0, // Start with 1.0
        color_offset: 0.0,
        ao_strength: 1.0,
        rim_strength: 0.5,
        rotation: Vec4::from(Quat::IDENTITY),
        julia: Vec4::new(0.35, 0.35, -0.35, 0.0), // last value 0, not used initially
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
    power: f32, // 4 bytes
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
    #[uniform(0)]
    palette_id: u32,
    #[uniform(0)]
    light_pos_x: f32,
    #[uniform(0)]
    light_pos_y: f32,
    #[uniform(0)]
    background_glow_intensity: f32,
    #[uniform(0)]
    color_scale: f32,
    #[uniform(0)]
    color_offset: f32,
    #[uniform(0)]
    ao_strength: f32,
    #[uniform(0)]
    rim_strength: f32,
    #[uniform(0)]
    rotation: Vec4,
    #[uniform(0)]
    julia: Vec4,
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
        material.resolution = Vec2::new(win.width(), win.height());

        // Animate the power parameter over time, goes 1->16->1 and loops
        if settings.animate_power {
            // normalized 0.0 to 1.0 sine
            let t = (0.5
                + 0.5 * (time.elapsed_secs_f64() * 0.1 * settings.power_speed as f64).sin())
                as f32;
            // Exponentially mapped because the power parameter has an exponential effect on the shape
            material.power = 16.0_f32.powf(t);
        }

        if settings.rotation_speed > 0.0 {
            let delta_rotation_y =
                Quat::from_rotation_y(settings.rotation_speed * time.delta_secs());
            let delta_rotation_x =
                Quat::from_rotation_x(settings.rotation_speed * time.delta_secs());

            let new_rotation =
                delta_rotation_y * delta_rotation_x * Quat::from_vec4(material.rotation);
            material.rotation = Vec4::from(new_rotation.normalize());
        }

        if settings.animate_zoom {
            material.camera_zoom =
                2.75 + ((time.elapsed_secs_f64() * settings.zoom_speed as f64).sin() as f32) * 0.25;
        }
    }
}

fn mouse_controls(
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion_evr: MessageReader<MouseMotion>,
    mut contexts: EguiContexts,
) {
    // If the mouse is over an egui area, don't rotate
    let ctx = contexts.ctx_mut().unwrap();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        return;
    }

    // On left mouse button drag, rotate the fractal
    if buttons.pressed(MouseButton::Left) {
        for ev in motion_evr.read() {
            let sensitivity = 0.005;

            let delta_yaw = Quat::from_rotation_y(ev.delta.x * sensitivity);
            let delta_pitch = Quat::from_rotation_x(-ev.delta.y * sensitivity);

            for (_, mat) in materials.iter_mut() {
                // Apply rotation directly to the material's Quat
                let current_quat = Quat::from_vec4(mat.rotation);
                let new_quat = current_quat * delta_yaw * delta_pitch;
                mat.rotation = Vec4::from(new_quat.normalize());
            }
        }
    }
}

fn manage_rendering_mode(
    mut winit_settings: ResMut<WinitSettings>,
    sim_settings: Res<SimSettings>,
) {
    // Check if anything requires continuous updates
    let is_animating = sim_settings.animate_zoom
        || sim_settings.animate_power
        || sim_settings.rotation_speed > 0.0;

    if is_animating {
        // If animating, render every frame
        winit_settings.focused_mode = UpdateMode::Continuous;
        winit_settings.unfocused_mode = UpdateMode::Continuous;
    } else {
        winit_settings.focused_mode = UpdateMode::Reactive {
            wait: Duration::from_secs_f64(1.0 / 60.0), // if focused, check 60 times per second
            react_to_device_events: false,
            react_to_user_events: false,
            react_to_window_events: false,
        };

        winit_settings.unfocused_mode = UpdateMode::Reactive {
            wait: Duration::from_secs(1), // if unfocused, check once per second
            react_to_device_events: false,
            react_to_user_events: false,
            react_to_window_events: false,
        };
    }
}

#[derive(Resource)]
struct SimSettings {
    rotation_speed: f32,
    animate_zoom: bool,
    zoom_speed: f32,
    animate_power: bool,
    power_speed: f32,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            rotation_speed: 0.2,
            animate_zoom: false,
            zoom_speed: 1.0,
            animate_power: false,
            power_speed: 1.0,
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

            for (_, mat) in materials.iter_mut() {
                // SHAPE SETTINGS
                ui.separator();
                ui.label("Shape");

                ui.add_enabled(
                    !settings.animate_power,
                    egui::Slider::new(&mut mat.power, 1.0..=16.0).text("Power"),
                );

                let mut iters = mat.mandel_iters as f32;
                if ui
                    .add(egui::Slider::new(&mut iters, 1.0..=50.0).text("Iterations"))
                    .changed()
                {
                    mat.mandel_iters = iters as u32;
                }

                // RENDERING SETTINGS
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

                // CAMERA SETTINGS
                ui.separator();
                ui.label("Camera");

                ui.add_enabled(
                    !settings.animate_zoom,
                    egui::Slider::new(&mut mat.camera_zoom, 0.1..=10.0).text("Zoom"),
                );

                ui.add(
                    egui::Slider::new(&mut settings.rotation_speed, 0.0..=1.0)
                        .text("Rotation Speed"),
                );

                // ANIMATION SETTINGS
                ui.separator();
                ui.heading("Animations");

                ui.checkbox(&mut settings.animate_power, "Auto-Animate Power");
                if settings.animate_power {
                    ui.indent("power_speed", |ui| {
                        ui.add(
                            egui::Slider::new(&mut settings.power_speed, 0.01..=4.0)
                                .text("Power Speed"),
                        );
                    });
                }

                ui.checkbox(&mut settings.animate_zoom, "Auto-Animate Zoom");
                if settings.animate_zoom {
                    ui.indent("zoom_speed", |ui| {
                        ui.add(
                            egui::Slider::new(&mut settings.zoom_speed, 0.1..=5.0)
                                .text("Zoom Speed"),
                        );
                    });
                }

                // VISUAL STYLE
                ui.separator();
                ui.heading("Visual Style");

                ui.add(
                    egui::Slider::new(&mut mat.background_glow_intensity, 0.0..=5.0).text("Background Brightness"),
                );

                ui.horizontal(|ui| {
                    ui.label("Color Palette");
                    egui::ComboBox::from_id_salt("palette_combo")
                        .selected_text(match mat.palette_id {
                            0 => "Standard",
                            1 => "Fire (Red/Yellow)",
                            2 => "Neon (Purple/Green)",
                            _ => "Unknown",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut mat.palette_id, 0, "Standard");
                            ui.selectable_value(&mut mat.palette_id, 2, "Fire");
                            ui.selectable_value(&mut mat.palette_id, 3, "Neon");
                        });
                });

                ui.add(
                    egui::Slider::new(&mut mat.color_scale, 0.1..=3.0)
                        .text("Color Scale")
                        .step_by(0.01),
                );
                ui.add(
                    egui::Slider::new(&mut mat.color_offset, 0.0..=1.0)
                        .text("Color Offset")
                        .step_by(0.005),
                );

                ui.separator();
                ui.heading("Lighting");
                ui.add(egui::Slider::new(&mut mat.light_pos_x, -10.0..=10.0).text("Light X"));
                ui.add(egui::Slider::new(&mut mat.light_pos_y, -10.0..=10.0).text("Light Y"));
                ui.add(
                    egui::Slider::new(&mut mat.ao_strength, 0.0..=5.0)
                        .text("Ambient Occlusion")
                        .step_by(0.01),
                );
                ui.add(
                    egui::Slider::new(&mut mat.rim_strength, 0.0..=2.0)
                        .text("Rim Lighting")
                        .step_by(0.01),
                );

                // JULIA FOLDING CONTROLS
                ui.separator();
                ui.heading("Julia Folding");

                // enable/disable toggle
                let mut is_julia = mat.julia.w > 0.5;
                if ui.checkbox(&mut is_julia, "Enable Julia Mode").changed() {
                    mat.julia.w = if is_julia { 1.0 } else { 0.0 };
                }

                // coordinate Sliders
                if is_julia {
                    ui.indent("julia_controls", |ui| {
                        ui.label("Constant K");
                        ui.add(egui::Slider::new(&mut mat.julia.x, -2.0..=2.0).step_by(0.005).text("X"));
                        ui.add(egui::Slider::new(&mut mat.julia.y, -2.0..=2.0).step_by(0.005).text("Y"));
                        ui.add(egui::Slider::new(&mut mat.julia.z, -2.0..=2.0).step_by(0.005).text("Z"));
                    });
                }
            }
        });
}
