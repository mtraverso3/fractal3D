use crate::material::{FractalMaterial, MandelbulbMaterial};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_egui::EguiContexts;

const MOVE_SPEED: f32 = 2.0;
const TURN_SPEED: f32 = 1.5;
const MOUSE_SENSITIVITY: f32 = 0.005;

/// Every key handled by `keyboard_controls`
const CONTROL_KEYS: [KeyCode; 10] = [
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::Space,
    KeyCode::ShiftLeft,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
];

#[derive(Resource)]
pub struct SimSettings {
    pub rotation_speed: f32,
    pub animate_zoom: bool,
    pub zoom_speed: f32,
    pub animate_power: bool,
    pub power_speed: f32,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            rotation_speed: 0.1,
            animate_zoom: false,
            zoom_speed: 1.0,
            animate_power: false,
            power_speed: 1.0,
        }
    }
}

impl SimSettings {
    fn is_animating(&self) -> bool {
        self.animate_zoom || self.animate_power || self.rotation_speed > 0.0
    }
}

/// Whether the app is currently redrawing every frame, or only when something happens
#[derive(Resource, Default)]
pub struct RenderLoop {
    continuous: bool,
}

impl RenderLoop {
    /// Frame delta for continuous motion. After an idle stretch the delta spans the whole
    /// wait, so the first frame back contributes nothing instead of making the camera jump.
    fn delta_secs(&self, time: &Time) -> f32 {
        if self.continuous {
            time.delta_secs()
        } else {
            0.0
        }
    }
}

/// Redraws every frame only while something is moving, otherwise waits for input.
/// Rendering the fractal is expensive, so a static view should not keep the GPU busy.
pub fn manage_render_loop(
    mut render_loop: ResMut<RenderLoop>,
    mut winit_settings: ResMut<WinitSettings>,
    settings: Res<SimSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    let continuous = settings.is_animating()
        || keys.any_pressed(CONTROL_KEYS)
        || buttons.pressed(MouseButton::Left);

    if continuous == render_loop.continuous {
        return;
    }
    render_loop.continuous = continuous;
    *winit_settings = if continuous {
        WinitSettings::continuous()
    } else {
        // Reacts to window and egui redraw requests, so the UI still updates immediately
        WinitSettings::desktop_app()
    };
}

/// Drives the time based animations: power, zoom and auto rotation
pub fn animate(
    time: Res<Time>,
    render_loop: Res<RenderLoop>,
    settings: Res<SimSettings>,
    fractal: Res<FractalMaterial>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) {
    if !settings.is_animating() {
        return;
    }
    let elapsed = time.elapsed_secs_f64();
    let dt = render_loop.delta_secs(&time);

    fractal.edit(&mut materials, |mat| {
        if settings.animate_power {
            // normalized 0.0 to 1.0 sine, goes 1 -> 16 -> 1 and loops
            let t = (0.5 + 0.5 * (elapsed * 0.1 * settings.power_speed as f64).sin()) as f32;
            // exponentially mapped because the power parameter has an exponential effect on the shape
            mat.power = 16.0_f32.powf(t);
        }

        if settings.rotation_speed > 0.0 {
            let angle = settings.rotation_speed * dt;
            mat.rotate(Quat::from_rotation_y(angle) * Quat::from_rotation_x(angle));
        }

        if settings.animate_zoom {
            mat.camera_zoom = 2.75 + ((elapsed * settings.zoom_speed as f64).sin() as f32) * 0.25;
        }
    });
}

/// Keeps the fullscreen quad and the shader resolution in sync with the window
pub fn fit_to_window(
    window: Single<&Window>,
    mut quad: Single<&mut Transform, With<MeshMaterial2d<MandelbulbMaterial>>>,
    fractal: Res<FractalMaterial>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) {
    let size = window.size();

    let scale = size.extend(1.0);
    if quad.scale != scale {
        quad.scale = scale;
    }

    fractal.edit(&mut materials, |mat| mat.resolution = size);
}

/// Handles keyboard input for controlling camera movement and rotation.
///
/// # Key Bindings
/// - W/A/S/D: Move forward/left/backward/right
/// - Space/Left Shift: Move up/down
/// - Arrow Keys: Rotate camera (about its own axes)
pub fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    render_loop: Res<RenderLoop>,
    fractal: Res<FractalMaterial>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) {
    if !keys.any_pressed(CONTROL_KEYS) {
        return;
    }

    let axis = |positive: KeyCode, negative: KeyCode| {
        keys.pressed(positive) as i8 as f32 - keys.pressed(negative) as i8 as f32
    };

    // camera space: +z forward, +x right, -y up
    let vertical = if keys.pressed(KeyCode::ShiftLeft) {
        1.0
    } else if keys.pressed(KeyCode::Space) {
        -1.0
    } else {
        0.0
    };
    let move_input = Vec3::new(
        axis(KeyCode::KeyD, KeyCode::KeyA),
        vertical,
        axis(KeyCode::KeyW, KeyCode::KeyS),
    );

    let dt = render_loop.delta_secs(&time);
    let yaw = axis(KeyCode::ArrowLeft, KeyCode::ArrowRight) * TURN_SPEED * dt;
    let pitch = axis(KeyCode::ArrowDown, KeyCode::ArrowUp) * TURN_SPEED * dt;

    fractal.edit(&mut materials, |mat| {
        if move_input != Vec3::ZERO {
            // move along the camera's own axes
            let local_move = move_input.normalize() * MOVE_SPEED * dt;
            mat.camera_position += mat.rotation().inverse() * local_move;
        }

        if yaw != 0.0 || pitch != 0.0 {
            mat.rotate(Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch));
        }
    });
}

/// Rotates the camera about the origin while the left mouse button is dragged
pub fn mouse_controls(
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut contexts: EguiContexts,
    fractal: Res<FractalMaterial>,
    mut materials: ResMut<Assets<MandelbulbMaterial>>,
) -> Result {
    // Motion over the egui window, or without the button held, is dropped rather than
    // left queued up and applied later
    let ctx = contexts.ctx_mut()?;
    if !buttons.pressed(MouseButton::Left)
        || ctx.is_pointer_over_area()
        || ctx.wants_pointer_input()
    {
        motion.clear();
        return Ok(());
    }

    fractal.edit(&mut materials, |mat| {
        for ev in motion.read() {
            let delta_yaw = Quat::from_rotation_y(-ev.delta.x * MOUSE_SENSITIVITY);
            let delta_pitch = Quat::from_rotation_x(ev.delta.y * MOUSE_SENSITIVITY);
            mat.rotate(delta_yaw * delta_pitch);
        }
    });
    Ok(())
}
