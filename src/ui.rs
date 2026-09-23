use crate::controls::SimSettings;
use crate::material::{FractalMaterial, MandelbulbMaterial};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Palette ids as understood by the shader, with their display names
const PALETTES: [(u32, &str); 3] = [
    (0, "Standard"),
    (1, "Fire (Red/Yellow)"),
    (2, "Neon (Purple/Green)"),
];

pub fn ui_controls(
    mut contexts: EguiContexts,
    fractal: Res<FractalMaterial>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
    mut settings: ResMut<SimSettings>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    // Edit a copy so the material is only re-uploaded when a value actually changed
    fractal.edit(&mut materials, |mat| {
        egui::Window::new("Mandelbulb Settings")
            .default_width(300.0)
            .show(ctx, |ui| settings_panel(ui, mat, &mut settings));
    });
    Ok(())
}

fn settings_panel(ui: &mut egui::Ui, mat: &mut MandelbulbMaterial, settings: &mut SimSettings) {
    ui.heading("Fractal Parameters");

    ui.separator();
    ui.label("Shape");
    ui.add_enabled(
        !settings.animate_power,
        egui::Slider::new(&mut mat.power, -2.0..=16.0).text("Power"),
    );
    ui.add(egui::Slider::new(&mut mat.mandel_iters, 1..=50).text("Iterations"));

    ui.separator();
    ui.label("Rendering Quality");
    ui.add(egui::Slider::new(&mut mat.ray_steps, 10..=300).text("Ray Steps"));
    ui.add(
        egui::Slider::new(&mut mat.hit_threshold, 0.0001..=0.01)
            .text("Threshold")
            .logarithmic(true),
    );
    ui.add(egui::Slider::new(&mut mat.max_dist, 10.0..=100.0).text("Max Dist"));

    ui.separator();
    ui.label("Camera");
    ui.add_enabled(
        !settings.animate_zoom,
        egui::Slider::new(&mut mat.camera_zoom, 0.1..=10.0).text("Zoom"),
    );
    ui.add(egui::Slider::new(&mut settings.rotation_speed, 0.0..=1.0).text("Rotation Speed"));

    ui.separator();
    ui.heading("Animations");
    ui.checkbox(&mut settings.animate_power, "Auto-Animate Power");
    if settings.animate_power {
        ui.indent("power_speed", |ui| {
            ui.add(egui::Slider::new(&mut settings.power_speed, 0.01..=4.0).text("Power Speed"));
        });
    }
    ui.checkbox(&mut settings.animate_zoom, "Auto-Animate Zoom");
    if settings.animate_zoom {
        ui.indent("zoom_speed", |ui| {
            ui.add(egui::Slider::new(&mut settings.zoom_speed, 0.1..=5.0).text("Zoom Speed"));
        });
    }

    ui.separator();
    ui.heading("Visual Style");
    ui.add(
        egui::Slider::new(&mut mat.background_glow_intensity, 0.0..=5.0)
            .text("Background Brightness"),
    );
    ui.horizontal(|ui| {
        ui.label("Color Palette");
        let selected = PALETTES
            .iter()
            .find(|(id, _)| *id == mat.palette_id)
            .map_or("Unknown", |(_, name)| *name);
        egui::ComboBox::from_id_salt("palette_combo")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (id, name) in PALETTES {
                    ui.selectable_value(&mut mat.palette_id, id, name);
                }
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
    ui.add(
        egui::Slider::new(&mut mat.fog_density, 0.0..=1.0)
            .text("Fog Density")
            .step_by(0.01),
    );

    ui.separator();
    ui.heading("Julia Folding");
    let mut is_julia = mat.julia.w > 0.5;
    if ui.checkbox(&mut is_julia, "Enable Julia Mode").changed() {
        mat.julia.w = if is_julia { 1.0 } else { 0.0 };
    }
    if is_julia {
        ui.indent("julia_controls", |ui| {
            ui.label("Constant K");
            for (i, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                ui.add(
                    egui::Slider::new(&mut mat.julia[i], -2.0..=2.0)
                        .step_by(0.005)
                        .text(axis),
                );
            }
        });
    }

    ui.separator();
    ui.heading("Performance");
    let mut supersampling = mat.supersampling_enabled > 0;
    if ui
        .checkbox(&mut supersampling, "Supersampling (2x2)")
        .changed()
    {
        mat.supersampling_enabled = supersampling as u32;
    }
}
