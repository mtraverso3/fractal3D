struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

// Must match the field order of `MandelbulbMaterial` in src/material.rs
struct MandelbulbMaterial {
    resolution: vec2<f32>,
    power: f32,
    ray_steps: u32,
    mandel_iters: u32,
    max_dist: f32,
    hit_threshold: f32,
    camera_zoom: f32,
    camera_position: vec3<f32>,
    camera_rotation: vec4<f32>,     // quaternion (x, y, z, w)
    palette_id: u32,                // 0=Standard, 1=Fire, 2=Neon
    light_pos_x: f32,
    light_pos_y: f32,
    background_glow_intensity: f32,
    color_scale: f32,               // stretches the gradient
    color_offset: f32,              // shifts the colors
    ao_strength: f32,
    rim_strength: f32,
    fog_density: f32,
    julia: vec4<f32>,               // xyz is the constant, w > 0.5 enables julia mode
    supersampling: u32,             // 0=off, 1=2x2 SSAA
};

@group(2) @binding(0)
var<uniform> material: MandelbulbMaterial;

// Every point further than this from the origin escapes on the first iteration of
// `sd_mandelbulb`, so the fractal surface lies entirely inside this radius
const BAILOUT: f32 = 2.0;
const TAU: f32 = 6.28318;

// Inigo Quilez's cosine palette function, makes nice smooth color gradients
// https://iquilezles.org/articles/palettes/
fn palette(t: f32) -> vec3<f32> {
    var a = vec3<f32>(0.5);
    var b = vec3<f32>(0.5);
    var c = vec3<f32>(1.0);
    var d = vec3<f32>(0.263, 0.416, 0.557);

    // Fire (Red/Yellow)
    if (material.palette_id == 1u) {
        a = vec3<f32>(0.5, 0.5, 0.0);
        b = vec3<f32>(0.5, 0.5, 0.0);
        c = vec3<f32>(0.1, 0.5, 0.0);
        d = vec3<f32>(0.0);
    }
    // Neon (Purple/Green)
    else if (material.palette_id == 2u) {
        c = vec3<f32>(2.0, 1.0, 0.0);
        d = vec3<f32>(0.5, 0.2, 0.25);
    }

    return a + b * cos(TAU * (c * t + d));
}

// Mandelbulb SDF, given current point p, estimates distance to the fractal surface along with orbit trap value
fn sd_mandelbulb(p: vec3<f32>) -> vec2<f32> {
    let power = material.power;
    // julia mode adds a fixed constant instead of the sample point
    let c = select(p, material.julia.xyz, material.julia.w > 0.5);

    var z = p;
    var dr = 1.0;
    var r = 0.0;
    var trap = 1e20; // minimum radius reached, used for coloring

    for (var i = 0u; i < material.mandel_iters; i++) {
        r = length(z);
        if (r > BAILOUT) { break; }
        trap = min(trap, r);

        // convert to polar, then scale and rotate the angles
        let theta = acos(z.z / r) * power;
        let phi = atan2(z.y, z.x) * power;

        // r^(power-1) is shared by the derivative and the new radius, saving a pow per iteration
        let r_pow = pow(r, power - 1.0);
        dr = r_pow * power * dr + 1.0;
        let zr = r_pow * r;

        // convert back to cartesian and add the constant
        let sin_theta = sin(theta);
        z = zr * vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos(theta)) + c;
    }

    // formula for distance estimation
    let dist = 0.5 * log(r) * r / dr;
    return vec2<f32>(dist, trap);
}

fn sphere_fold(z: vec3<f32>) -> vec3<f32> {
    let min_r = 0.5;
    let fixed_r = 1.0;
    let r2 = dot(z, z);
    if (r2 < min_r) {
        return z * (fixed_r / min_r);
    } else if (r2 < fixed_r) {
        return z * (fixed_r / r2);
    }
    return z;
}

fn box_fold(z: vec3<f32>) -> vec3<f32> {
    let folding_limit = 1.0;
    return clamp(z, vec3<f32>(-folding_limit), vec3<f32>(folding_limit)) * 2.0 - z;
}

// Work in progress, swap it in via `map_full`. Note that the escape early-out in
// `render_ray` assumes the Mandelbulb's BAILOUT and must be removed for the Mandelbox.
fn sd_mandelbox(p: vec3<f32>) -> vec2<f32> {
    let scale = material.power;
    let offset = select(p, material.julia.xyz, material.julia.w > 0.5);

    var z = p;
    var dr = 1.0;
    var trap = 1e20;

    for (var i = 0u; i < material.mandel_iters; i++) {
        z = sphere_fold(box_fold(z));
        z = z * scale + offset;
        dr = dr * abs(scale) + 1.0;
        trap = min(trap, length(z));
    }

    return vec2<f32>(length(z) / abs(dr), trap);
}

// Rotate vector p by the inverse/conjugate of quaternion q
fn rotate_vector_inverse(p: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    let u = -q.xyz;
    return p + 2.0 * cross(u, cross(u, p) + q.w * p);
}

// Distance and orbit trap of the active fractal
fn map_full(p: vec3<f32>) -> vec2<f32> {
    return sd_mandelbulb(p);
}

// Distance only, lets the compiler drop the orbit trap bookkeeping
fn map(p: vec3<f32>) -> f32 {
    return map_full(p).x;
}

// Surface normal using the tetrahedron technique: 4 SDF evaluations instead of the 6
// needed for central differences, sampled at the same distance from p
// see: https://iquilezles.org/articles/normalsSDF/
fn calculate_normal(p: vec3<f32>) -> vec3<f32> {
    let h = material.hit_threshold * 0.5 * 0.5773; // 1/sqrt(3), keeps the sample radius at hit_threshold/2
    let k = vec2<f32>(1.0, -1.0);
    return normalize(
        k.xyy * map(p + k.xyy * h) +
        k.yyx * map(p + k.yyx * h) +
        k.yxy * map(p + k.yxy * h) +
        k.xxx * map(p + k.xxx * h)
    );
}

fn shade(p: vec3<f32>, ro: vec3<f32>, trap: f32, t: f32, step_frac: f32) -> vec3<f32> {
    let normal = calculate_normal(p);

    // combine orbit trap and steps for more variation
    let albedo = palette((trap + step_frac) * material.color_scale + material.color_offset);

    let light_pos = vec3<f32>(material.light_pos_x, material.light_pos_y, -3.0);
    let light_dir = normalize(light_pos - p);
    let view_dir = normalize(ro - p);

    // basic diffuse lighting based on angle to light
    let diff = max(dot(normal, light_dir), 0.0);

    // specular, see https://en.wikipedia.org/wiki/Blinn%E2%80%93Phong_reflection_model
    let half_vec = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, half_vec), 0.0), 32.0);

    // rim lighting, edges perpendicular to view get a glow
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 4.0);

    // fake ambient occlusion based on number of steps taken to hit surface
    let ao = 1.0 - step_frac * material.ao_strength;

    let ambient = vec3<f32>(0.1) * albedo;
    let diffuse_light = albedo * diff * vec3<f32>(1.0, 0.9, 0.8);
    let specular_light = vec3<f32>(spec * 0.8);
    let rim_light = vec3<f32>(0.0, 0.5, 1.0) * rim * material.rim_strength;

    let col = (ambient + diffuse_light + specular_light + rim_light) * ao;

    // some fog based on distance
    return mix(col, vec3<f32>(0.01, 0.01, 0.02), 1.0 - exp(-material.fog_density * t));
}

// ro is the ray origin in world space, to_origin the unit vector from ro towards the origin
fn render_ray(uv: vec2<f32>, ro: vec3<f32>, to_origin: vec3<f32>) -> vec3<f32> {
    // ray direction in camera space (focal length 1.5), then rotated to world space
    let rd = rotate_vector_inverse(normalize(vec3<f32>(uv, 1.5)), material.camera_rotation);

    let steps = material.ray_steps;
    var t = 0.0;

    for (var i = 0u; i < steps; i++) {
        let p = ro + rd * t;

        // Outside the bailout sphere the SDF is >= 0.5*ln(2)*2, far above any hit threshold, and
        // a ray moving away from the origin only gets further out, so it can never hit
        if (dot(p, p) > BAILOUT * BAILOUT && dot(p, rd) > 0.0) { break; }

        let data = map_full(p); // .x = dist, .y = trap
        if (data.x < material.hit_threshold) {
            return shade(p, ro, data.y, t, f32(i) / f32(steps));
        }

        t += data.x;
        if (t > material.max_dist) { break; }
    }

    // missed: background gradient with a halo around the fractal
    let bg = exp(uv.y - 2.0) * vec3<f32>(0.2, 0.4, 0.8) * material.background_glow_intensity;
    let halo = clamp(dot(to_origin, rd), 0.0, 1.0);
    return bg + vec3<f32>(0.02, 0.02, 0.08) * pow(halo, 17.0);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = material.resolution.x / material.resolution.y;

    // camera setup is the same for every sample, so do it once
    let ro = material.camera_position
        + rotate_vector_inverse(vec3<f32>(0.0, 0.0, -material.camera_zoom), material.camera_rotation);
    let to_origin = normalize(-ro);

    var col: vec3<f32>;
    if (material.supersampling > 0u) {
        // quarter pixel offsets in uv space for 2x2 supersampling
        let q = 0.25 / material.resolution;
        let offsets = array<vec2<f32>, 4>(
            vec2<f32>(-q.x, -q.y),
            vec2<f32>( q.x, -q.y),
            vec2<f32>(-q.x,  q.y),
            vec2<f32>( q.x,  q.y)
        );

        var total = vec3<f32>(0.0);
        for (var i = 0; i < 4; i++) {
            var uv = (in.uv + offsets[i]) * 2.0 - 1.0;
            uv.x *= aspect;
            total += render_ray(uv, ro, to_origin);
        }
        col = total * 0.25;
    } else {
        var uv = in.uv * 2.0 - 1.0;
        uv.x *= aspect;
        col = render_ray(uv, ro, to_origin);
    }

    // Gamma correction
    return vec4<f32>(pow(col, vec3<f32>(0.5545)), 1.0); // approx 1/2.2 + 0.1
}
