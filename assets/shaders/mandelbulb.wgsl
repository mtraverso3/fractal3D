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
    camera_zoom: f32,     // 4 bytes
    palette_id: u32,   // 0=Standard, 1=Ice, 2=Fire, 3=Neon
    light_pos_x: f32,  // Move the light left/right
    light_pos_y: f32,  // Move the light up/down
    glow_intensity: f32, // Intensity of the background glow
    color_scale: f32,  // Stretches the gradient
    color_offset: f32, // Shifts the colors
    rotation_angle: f32,
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
        z += p;
    }

    // formula for distance estimation
    let dist = 0.5 * log(r) * r / dr;
    return vec2<f32>(dist, trap);
}

// Wrapper to handle rotation and return full data
fn map_full(p: vec3<f32>) -> vec2<f32> {
    let rotated_p = rotate_y(p, material.rotation_angle);
    let rotated_p2 = rotate_x(rotated_p, material.rotation_angle);
    return sd_mandelbulb(rotated_p2);
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
    let ro = vec3<f32>(0.0, 0.0, -material.camera_zoom); // ray origin
    let rd = normalize(vec3<f32>(uv, 1.5)); // ray direction (focal length 1.5)

    var t = 0.0; // distance along the ray

    // background color, simple gradient with halo effect
    let bg = exp(uv.y - 2.0) * vec3<f32>(0.2, 0.4, 0.8) * material.glow_intensity;
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
            let ao = 1.0 - (f32(i) / f32(steps));

            // Combine lighting components
            let ambient = vec3<f32>(0.1) * albedo;
            let diffuse_light = albedo * diff * vec3<f32>(1.0, 0.9, 0.8);
            let specular_light = vec3<f32>(1.0) * spec * 0.8;
            let rim_light = vec3<f32>(0.0, 0.5, 1.0) * rim * 0.5;

            col = (ambient + diffuse_light + specular_light + rim_light) * ao;

            // some fog based on distance
            col = mix(col, vec3<f32>(0.01, 0.01, 0.02), 1.0 - exp(-0.05 * t * t));

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
    // size of one pixel in UV space, currently hardcoded estimate
    // ideally, we do let pixel_size = 1.0 / vec2<f32>(screen_width, screen_height);
    let px = 0.0008;

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
        let sub_uv = (sub_uv_raw * 2.0) - 1.0;

        total_color += render_ray(sub_uv);
    }

    // average the samples
    let avg_col = total_color / 4.0;

    // Gamma correction
    let final_col = pow(avg_col, vec3<f32>(0.5545)); // approx 1/2.2 + 0.1

    return vec4<f32>(final_col, 1.0);
}