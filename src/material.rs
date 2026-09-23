use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
    sprite_render::Material2d,
};

/// Uniforms for the raymarching shader. Field order must match the WGSL struct.
#[derive(Asset, TypePath, AsBindGroup, Clone, PartialEq)]
pub struct MandelbulbMaterial {
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(0)]
    pub power: f32,
    #[uniform(0)]
    pub ray_steps: u32,
    #[uniform(0)]
    pub mandel_iters: u32,
    #[uniform(0)]
    pub max_dist: f32,
    #[uniform(0)]
    pub hit_threshold: f32,
    #[uniform(0)]
    pub camera_zoom: f32,
    #[uniform(0)]
    pub camera_position: Vec3,
    /// Quaternion stored as a `Vec4` so it maps directly onto a WGSL `vec4<f32>`
    #[uniform(0)]
    pub camera_rotation: Vec4,
    #[uniform(0)]
    pub palette_id: u32,
    #[uniform(0)]
    pub light_pos_x: f32,
    #[uniform(0)]
    pub light_pos_y: f32,
    #[uniform(0)]
    pub background_glow_intensity: f32,
    #[uniform(0)]
    pub color_scale: f32,
    #[uniform(0)]
    pub color_offset: f32,
    #[uniform(0)]
    pub ao_strength: f32,
    #[uniform(0)]
    pub rim_strength: f32,
    #[uniform(0)]
    pub fog_density: f32,
    /// xyz is the julia constant, w > 0.5 enables julia mode
    #[uniform(0)]
    pub julia: Vec4,
    #[uniform(0)]
    pub supersampling_enabled: u32,
}

impl MandelbulbMaterial {
    pub fn new(resolution: Vec2) -> Self {
        Self {
            resolution,
            power: 8.0,
            ray_steps: 220,
            mandel_iters: 10,
            max_dist: 20.0,
            hit_threshold: 0.0025,
            camera_zoom: 2.5,
            camera_position: Vec3::ZERO,
            camera_rotation: Vec4::from(Quat::IDENTITY),
            palette_id: 0,
            light_pos_x: 8.0,
            light_pos_y: 10.0,
            background_glow_intensity: 0.0,
            color_scale: 0.95,
            color_offset: 0.05,
            ao_strength: 1.2,
            rim_strength: 0.1,
            fog_density: 0.05,
            julia: Vec4::new(0.35, 0.35, -0.35, 0.0),
            supersampling_enabled: 0,
        }
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_vec4(self.camera_rotation)
    }

    /// Applies `delta` on top of the current camera rotation
    pub fn rotate(&mut self, delta: Quat) {
        self.camera_rotation = Vec4::from((delta * self.rotation()).normalize());
    }
}

impl Material2d for MandelbulbMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/mandelbulb.wgsl".into()
    }
}

/// Handle to the single fractal material drawn on the fullscreen quad
#[derive(Resource)]
pub struct FractalMaterial(pub Handle<MandelbulbMaterial>);

impl FractalMaterial {
    /// Runs `edit` on a copy of the material and only writes it back if something changed.
    ///
    /// Mutable access to an asset marks it as modified, which makes Bevy re-upload the
    /// uniforms and rebuild the bind group, so untouched frames should not take it.
    pub fn edit(
        &self,
        materials: &mut Assets<MandelbulbMaterial>,
        edit: impl FnOnce(&mut MandelbulbMaterial),
    ) {
        let Some(current) = materials.get(&self.0) else {
            return;
        };
        let mut edited = current.clone();
        edit(&mut edited);
        if edited != *current
            && let Some(material) = materials.get_mut(&self.0)
        {
            *material = edited;
        }
    }
}
