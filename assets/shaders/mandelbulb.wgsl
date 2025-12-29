struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct MandelbulbMaterial {
    resolution: vec2<f32>, // 8 bytes
    power: f32,            // 4 bytes
    ray_steps: u32,        // 4 bytes
    mandel_iters: u32,     // 4 bytes
    max_dist: f32,         // 4 bytes
    hit_threshold: f32,    // 4 bytes
    camera_zoom: f32,      // 4 bytes
    camera_position: vec3<f32>, // x, y, z
    camera_rotation: vec4<f32>, // Quaternion rotation (x, y, z, w)
    palette_id: u32,   // 0=Standard, 1=Ice, 2=Fire, 3=Neon
    light_pos_x: f32,  // Move the light left/right
    light_pos_y: f32,  // Move the light up/down
    background_glow_intensity: f32, // Intensity of the background glow
    color_scale: f32,  // Stretches the gradient
    color_offset: f32, // Shifts the colors
    ao_strength: f32,  // Ambient occlusion strength
    rim_strength: f32, // Rim lighting strength
    fog_density: f32,  // Fog density
    julia: vec4<f32>, // xyz are the constant, w is enabled flag
    supersampling: u32, // 0=off, 1=2x2 SSAA
};

@group(2) @binding(0)
var<uniform> material: MandelbulbMaterial;

// rotation helper, rotates point p around Y axis by angle in radians
fn rotate_y(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        c * p.x + s * p.z,
        p.y,
        -s * p.x + c * p.z
    );
}

// rotation helper, rotates point p around X axis by angle in radians
fn rotate_x(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        p.x,
        c * p.y - s * p.z,
        s * p.y + c * p.z
    );
}

// Inigo Quilez's cosine palette function, makes nice smooth color gradients
// https://iquilezles.org/articles/palettes/
fn palette(t: f32) -> vec3<f32> {
    var a = vec3<f32>(0.5);
    var b = vec3<f32>(0.5);
    var c = vec3<f32>(1.0);
    var d = vec3<f32>(0.263, 0.416, 0.557);

    // Standard
    if (material.palette_id == 0u) {
        d = vec3<f32>(0.263, 0.416, 0.557);
    }
    // Fire (Red/Yellow)
    else if (material.palette_id == 1u) {
        a = vec3<f32>(0.500, 0.500, 0.000);
        b = vec3<f32>(0.500, 0.500, 0.000);
        c = vec3<f32>(0.100, 0.500, 0.000);
        d = vec3<f32>(0.000, 0.000, 0.000);
    }
    // Neon (Purple/Green)
    else if (material.palette_id == 2u) {
        a = vec3<f32>(0.5, 0.5, 0.5);
        b = vec3<f32>(0.5, 0.5, 0.5);
        c = vec3<f32>(2.0, 1.0, 0.0);
        d = vec3<f32>(0.5, 0.2, 0.25);
    }

    return a + b * cos(6.28318 * (c * t + d));
}


// Mandelbulb SDF, given current point p, estimates distance to the fractal surface along with orbit trap value
fn sd_mandelbulb(p: vec3<f32>) -> vec2<f32> {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    let power = 8.0;

    var trap = 1e20; // Initialize trap to a large value, will store minimum radius reached

    for (var i = 0u; i < material.mandel_iters; i++) {
        r = length(z);
        if (r > 2.0) { break; }

        // Update Trap, keeping minimum radius reached
        var c = p;
        if (material.julia.w > 0.5) {
            c = material.julia.xyz;
        }
        trap = min(trap, r);

        // convert to polar
        var theta = acos(z.z / r);
        var phi = atan2(z.y, z.x);

        // calculate the derivative, needed at end for distance estimation
        dr = pow(r, material.power - 1.0) * material.power * dr + 1.0;

        // scale and rotate the point
        let zr = pow(r, material.power);
        theta = theta * material.power;
        phi = phi * material.power;

        // convert back to cartesian
        z = zr * vec3<f32>(
            sin(theta) * cos(phi),
            sin(theta) * sin(phi),
            cos(theta)
        );

        // add the constant c
        z += c;
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
fn sd_mandelbox(p: vec3<f32>) -> vec2<f32> {
    var z = p;
    var dr = 1.0;

    let scale = material.power;

    var offset = p;
    if (material.julia.w > 0.5) {
        offset = material.julia.xyz;
    }

    var trap = 1e20;

    for (var i = 0u; i < material.mandel_iters; i++) {
        z = box_fold(z);
        z = sphere_fold(z);

        z = z * scale + offset;
        dr = dr * abs(scale) + 1.0;

        // Trap for coloring
        trap = min(trap, length(z));
    }

    let r = length(z);
    return vec2<f32>(r / abs(dr), trap);
}

// Rotate vector p by quaternion q
fn rotate_vector(p: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    return p + 2.0 * cross(q.xyz, cross(q.xyz, p) + q.w * p);
}

// Rotate vector p by the inverse/conjugate of quaternion q
fn rotate_vector_inverse(p: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    // Conjugate of quaternion, negate xyz, keep w
    let q_conj = vec4<f32>(-q.xyz, q.w);
    return rotate_vector(p, q_conj);
}

// Wrapper to handle rotation and return full data (not anymore, todo:remove)
fn map_full(p: vec3<f32>) -> vec2<f32> {
    return sd_mandelbulb(p);
//    return sd_mandelbox(p);
}

// Wrapper that just returns distance (cheaper for normals)
fn map(p: vec3<f32>) -> f32 {
    return map_full(p).x;
}

// Calculate the normal at point p using "central differences"
// see: https://iquilezles.org/articles/normalsSDF/
fn calculate_normal(p: vec3<f32>) -> vec3<f32> {
    let e = material.hit_threshold * 0.5;
    return normalize(vec3<f32>(
        map(p + vec3<f32>(e, 0.0, 0.0)) - map(p - vec3<f32>(e, 0.0, 0.0)),
        map(p + vec3<f32>(0.0, e, 0.0)) - map(p - vec3<f32>(0.0, e, 0.0)),
        map(p + vec3<f32>(0.0, 0.0, e)) - map(p - vec3<f32>(0.0, 0.0, e))
    ));
}

fn render_ray(uv: vec2<f32>) -> vec3<f32> {
    // Camera Setup
    let local_offset = vec3<f32>(0.0, 0.0, -material.camera_zoom);

    // rotate camera offset by the rotation quaternion
    let rotated_offset = rotate_vector_inverse(local_offset, material.camera_rotation);
    let ro = material.camera_position + rotated_offset; // ray origin in world space

    // ray direction in camera space, then rotate to world space
    let local_rd = normalize(vec3<f32>(uv, 1.5)); // ray direction (focal length 1.5)
    let rd = rotate_vector_inverse(local_rd, material.camera_rotation);

    var t = 0.0; // distance along the ray

    // background color, simple gradient with halo effect
    let bg = exp(uv.y - 2.0) * vec3<f32>(0.2, 0.4, 0.8) * material.background_glow_intensity;
    let halo = clamp(dot(normalize(vec3<f32>(-ro.x, -ro.y, -ro.z)), rd), 0.0, 1.0);
    var col = bg + vec3<f32>(0.02, 0.02, 0.08) * pow(halo, 17.0);

    let steps = material.ray_steps;

    // ray march loop
    for (var i = 0u; i < steps; i++) {
        // current position along the ray
        let p = ro + rd * t;
        let data = map_full(p); // .x = dist, .y = trap
        let d = data.x;

        // hit condition, close enough to the surface
        if (d < material.hit_threshold) {
            let normal = calculate_normal(p);
            let trap = data.y; // The orbit trap value

            let raw_val = trap + (f32(i) / f32(steps)); // combine orbit trap and steps for more variation
            let color_variation = (raw_val * material.color_scale) + material.color_offset;
            let albedo = palette(color_variation);

            // lighting Setup
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
            let ao = 1.0 - (f32(i) / f32(steps)) * material.ao_strength;

            // Combine lighting components
            let ambient = vec3<f32>(0.1) * albedo;
            let diffuse_light = albedo * diff * vec3<f32>(1.0, 0.9, 0.8);
            let specular_light = vec3<f32>(1.0) * spec * 0.8;
            let rim_light = vec3<f32>(0.0, 0.5, 1.0) * rim * material.rim_strength;

            col = (ambient + diffuse_light + specular_light + rim_light) * ao;

            // some fog based on distance
            col = mix(col, vec3<f32>(0.01, 0.01, 0.02), 1.0 - exp(-material.fog_density * t));

            break;
        }

        t += d; // march the ray

        // ray exceeded max distance
        if (t > material.max_dist) { break; }
    }

    return col;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = material.resolution.x / material.resolution.y;
    var col: vec3<f32>;

    if (material.supersampling > 0u) {
        // size of one pixel in UV space, currently hardcoded estimate
        // ideally, we do let pixel_size = 1.0 / vec2<f32>(screen_width, screen_height);
        let px = vec2<f32>(1.0 / material.resolution.x, 1.0 / material.resolution.y);

        // 2x2 super sampling offsets
        let offsets = array<vec2<f32>, 4>(
            vec2<f32>(-0.25, -0.25),
            vec2<f32>( 0.25, -0.25),
            vec2<f32>(-0.25,  0.25),
            vec2<f32>( 0.25,  0.25)
        );

        var total_color = vec3<f32>(0.0);

        // grab colors from each sub-pixel sample
        for (var i = 0; i < 4; i++) {
            // calculate the specific sub-pixel UV
            let sub_uv_raw = in.uv + (offsets[i] * px);

            // remap to [-1, 1]
            var sub_uv = (sub_uv_raw * 2.0) - 1.0;
            sub_uv.x *= aspect;

            total_color += render_ray(sub_uv);
        }
        // average the samples
        col = total_color / 4.0;
    } else {
        var uv = (in.uv * 2.0) - 1.0;
        uv.x *= aspect;
        col = render_ray(uv);
    }

    // Gamma correction
    let final_col = pow(col, vec3<f32>(0.5545)); // approx 1/2.2 + 0.1
    return vec4<f32>(final_col, 1.0);
}