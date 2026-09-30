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
    /// Hit threshold in pixels is `1 / detail`
    #[uniform(0)]
    pub detail: f32,
    /// Scales every raymarching step, below 1 for formulas that overestimate distance
    #[uniform(0)]
    pub step_factor: f32,
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
            detail: 1.0,
            step_factor: 1.0,
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

    /// Where the rays start, matching the shader's `ro`
    pub fn eye(&self) -> Vec3 {
        self.camera_position + self.rotation().inverse() * Vec3::new(0.0, 0.0, -self.camera_zoom)
    }

    /// Estimated distance from `p` to the fractal surface, the same estimate the shader marches with
    pub fn distance(&self, p: Vec3) -> f32 {
        const BAILOUT: f32 = 2.0;
        let c = if self.julia.w > 0.5 {
            self.julia.xyz()
        } else {
            p
        };

        let mut z = p;
        let mut dr = 1.0;
        let mut r = 0.0;
        for _ in 0..self.mandel_iters {
            r = z.length();
            if r > BAILOUT {
                break;
            }
            let theta = z.xy().length().atan2(z.z) * self.power;
            let phi = z.y.atan2(z.x) * self.power;
            let r_pow = r.powf(self.power - 1.0);
            dr = r_pow * self.power * dr + 1.0;
            z = r_pow
                * r
                * Vec3::new(
                    theta.sin() * phi.cos(),
                    theta.sin() * phi.sin(),
                    theta.cos(),
                )
                + c;
        }

        if r == 0.0 {
            return 0.0;
        }
        (0.5 * r.ln() * r / dr).max(0.0)
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
