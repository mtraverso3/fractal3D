struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct MandelbulbMaterial {
    resolution: vec2<f32>, // 8 bytes
    time: f32,             // 4 bytes
    power: f32,            // 4 bytes
    speed: f32,            // 4 bytes
    ray_steps: u32,        // 4 bytes
    mandel_iters: u32,     // 4 bytes
    max_dist: f32,         // 4 bytes
    hit_threshold: f32,    // 4 bytes
    camera_zoom: f32,     // 4 bytes
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

// Mandelbulb SDF, given current point p, estimates distance to the fractal surface
fn sd_mandelbulb(p: vec3<f32>) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    let power = 8.0;

    // iterating z = z^8 + c
    for (var i = 0u; i < material.mandel_iters; i++) {
        r = length(z);
        if (r > 2.0) { break; }

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
    return 0.5 * log(r) * r / dr;
}

// calculate distance to surface at point p after applying any transformations
fn map(p: vec3<f32>) -> f32 {
    let rotated_p = rotate_y(p, material.time * material.speed); // rotation transformation over time
    let rotated_p2 = rotate_x(rotated_p, material.time * material.speed);
    return sd_mandelbulb(rotated_p2);
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

fn render_ray(uv: vec2<f32>, time: f32) -> vec3<f32> {
    // Camera Setup
    let ro = vec3<f32>(0.0, 0.0, -material.camera_zoom); // ray origin
    let rd = normalize(vec3<f32>(uv, 1.5)); // ray direction (focal length 1.5)

    var t = 0.0; // distance along the ray

    // background color, simple gradient with halo effect
    let bg = exp(uv.y - 2.0) * vec3<f32>(0.2, 0.4, 0.8);
    let halo = clamp(dot(normalize(vec3<f32>(-ro.x, -ro.y, -ro.z)), rd), 0.0, 1.0);
    var col = bg + vec3<f32>(0.02, 0.02, 0.08) * pow(halo, 17.0);

    let steps = material.ray_steps;

    // ray march loop
    for (var i = 0u; i < steps; i++) {
        // current position along the ray
        let p = ro + rd * t;
        let d = map(p);

        // hit condition, close enough to the surface
        if (d < material.hit_threshold) {
            let normal = calculate_normal(p);

            // Lighting
            let light_dir = normalize(vec3<f32>(0.8, 0.8, -1.0));
            let diffuse = max(dot(normal, light_dir), 0.0);

            // Coloring by mixing orange and blue based on normal
            col = mix(vec3<f32>(0.1, 0.2, 0.4), vec3<f32>(1.0, 0.6, 0.2), diffuse);

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

        total_color += render_ray(sub_uv, material.time);
    }

    var avg_col = total_color / 4.0;

    return vec4<f32>(avg_col, 1.0);
}