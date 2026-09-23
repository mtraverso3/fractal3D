mod controls;
mod material;
mod ui;

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy::winit::WinitSettings;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use controls::{RenderLoop, SimSettings};
use material::{FractalMaterial, MandelbulbMaterial};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            Material2dPlugin::<MandelbulbMaterial>::default(),
        ))
        .init_resource::<SimSettings>()
        .init_resource::<RenderLoop>()
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    controls::keyboard_controls,
                    controls::mouse_controls,
                    controls::animate,
                    controls::fit_to_window,
                ),
                // runs last so the other systems see the mode that was active for this frame
                controls::manage_render_loop,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, ui::ui_controls)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
    window: Single<&Window>,
) {
    // The whole image comes from the fullscreen quad's fragment shader, so multisampling
    // would only add render target memory and a resolve pass without changing any pixel
    commands.spawn((Camera2d, Msaa::Off));

    let material = materials.add(MandelbulbMaterial::new(window.size()));
    commands.insert_resource(FractalMaterial(material.clone()));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(material),
        Transform::from_scale(window.size().extend(1.0)),
    ));
}
