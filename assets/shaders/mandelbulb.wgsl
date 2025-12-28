struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct MandelbulbMaterial {
    time: f32,
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

// Mandelbulb SDF, given current point p, estimates distance to the fractal surface
fn sd_mandelbulb(p: vec3<f32>) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    let power = 8.0;

    // iterating z = z^8 + c
    for (var i = 0; i < 15; i++) {
        r = length(z);
        if (r > 2.0) { break; }

        // convert to polar
        var theta = acos(z.z / r);
        var phi = atan2(z.y, z.x);

        // calculate the derivative, needed at end for distance estimation
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // scale and rotate the point
        let zr = pow(r, power);
        theta = theta * power;
        phi = phi * power;

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
    let rotated_p = rotate_y(p, material.time * 0.5); // rotation transformation over time
    return sd_mandelbulb(rotated_p);
}

// Calculate the normal at point p using "central differences"
// see: https://iquilezles.org/articles/normalsSDF/
fn calculate_normal(p: vec3<f32>) -> vec3<f32> {
    let e = 0.001; // small epsilon for numerical derivative
    return normalize(vec3<f32>(
        map(p + vec3<f32>(e, 0.0, 0.0)) - map(p - vec3<f32>(e, 0.0, 0.0)),
        map(p + vec3<f32>(0.0, e, 0.0)) - map(p - vec3<f32>(0.0, e, 0.0)),
        map(p + vec3<f32>(0.0, 0.0, e)) - map(p - vec3<f32>(0.0, 0.0, e))
    ));
}

fn render_ray(uv: vec2<f32>, time: f32) -> vec3<f32> {
    // Camera Setup
    let ro = vec3<f32>(0.0, 0.0, -3.5);
    let rd = normalize(vec3<f32>(uv, 1.5));

    var t = 0.0; // distance along the ray
    var col = vec3<f32>(0.05, 0.05, 0.08); // Background color

    // ray march loop
    for (var i = 0; i < 100; i++) {
        // current position along the ray
        let p = ro + rd * t;
        let d = map(p);

        // hit condition, close enough to the surface
        if (d < 0.002) {
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
        if (t > 20.0) { break; }
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