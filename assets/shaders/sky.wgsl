// The sky behind everything (under the light overlay, which grades it by
// the hour): from its colour at the horizon up to a deeper one overhead in
// a few bands, dithered only where one meets the next; low, the horizon on
// the sun's side warm and a violet belt across the far side (dawn, dusk);
// round the sun or moon a glow, stepped into rings. In sRGB (the sky's
// colours), out linear.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Sky {
    // rgb overhead; w how much the surface shows (the alpha).
    zenith: vec4<f32>,
    // rgb at the horizon; w the horizon's world y.
    horizon: vec4<f32>,
    // rgb the glow; w its strength.
    glow: vec4<f32>,
    // rgb the far belt; w how golden the hour.
    belt: vec4<f32>,
    // rgb the sun side's horizon.
    warm: vec4<f32>,
    // xy the disc's world position.
    sun: vec4<f32>,
    // xy the view's bottom left (world), zw its half size.
    view: vec4<f32>,
    // The aurora: x how strong (0: none), y its foot's world y as it seems,
    // z how far it's moved with the camera, w the time (s).
    aurora: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> s: Sky;

fn bayer(x: u32, y: u32) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return (m[(y & 3u) * 4u + (x & 3u)] + 0.5) / 16.0;
}

// One of the aurora's curtains (k 0 the near one, 1 a fainter one farther
// off and higher) at `x` cells from the view's left, `y` world: green low,
// violet up its folds, rippling, finely rayed; its colour and how strong.
fn curtain(x: f32, y: f32, k: f32) -> vec4<f32> {
    let t = s.aurora.w;
    let u = x + s.aurora.z * (1.0 - 0.4 * k) + k * 2300.0;
    let ph = k * 3.7;
    let v = 0.5 + 0.35 * sin(u / 1400.0 + t * 0.03 + ph) + 0.15 * sin(u / 380.0 - t * 0.05 + ph);
    let fold = smoothstep(0.0, 1.0, clamp((v - 0.3) / 0.35, 0.0, 1.0));
    let edge = s.aurora.y + k * 90.0 + 55.0 * sin(u / 300.0 + t * 0.05 + ph) + 25.0 * sin(u / 110.0 - t * 0.09 + ph) + 8.0 * sin(u / 37.0 + t * 0.21);
    let dy = y - edge;
    // (Up from its foot over a few dozen cells, brightest a little over
    // it, thinning out up its height to nothing.)
    let rise = smoothstep(-28.0, 18.0, dy);
    let thin = 1.0 - smoothstep(0.0, 1.0, clamp(dy / 320.0, 0.0, 1.0));
    let fall = rise * thin * exp(-max(dy, 0.0) / 170.0);
    // (Rays: a soft grain along it, never gaps; brighter bands drifting
    // through.)
    let grain = 0.5 + 0.25 * sin(u / 3.1 + 2.0 * sin(u / 47.0 + t * 0.35)) + 0.25 * sin(u / 7.3 - t * 0.2);
    let ray = (0.82 + 0.18 * grain) * (0.7 + 0.3 * sin(u / 80.0 + t * 0.15 + ph));
    let up = smoothstep(0.0, 1.0, clamp(dy / 180.0, 0.0, 1.0));
    let rgb = vec3<f32>(0.35 + 0.4 * up, 1.0 - 0.65 * up, 0.55 + 0.4 * up);
    return vec4<f32>(rgb, fold * fall * ray * (1.0 - 0.5 * k));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xy;
    let half = s.view.zw;
    // (Dithered on the view's cells: the sky's fixed to the view.)
    let cell = vec2<u32>(max(floor(p - s.view.xy), vec2<f32>(0.0)));
    let d = bayer(cell.x, cell.y);
    // How high (0 the horizon .. 1 overhead, a view and a half up), in bands.
    let e = clamp((p.y - s.horizon.w) / (half.y * 1.6), 0.0, 1.0);
    let bands = 8.0;
    let eq = floor(e * bands + 0.25 + 0.5 * d) / bands;
    var c = mix(s.horizon.rgb, s.zenith.rgb, smoothstep(0.0, 1.0, eq));
    // Low: the sun's side warm, the far side's belt (when it's golden).
    let side = exp(-abs(p.x - s.sun.x) / (half.x * 1.1));
    let low = 1.0 - smoothstep(0.0, 0.55, eq);
    c = mix(c, s.warm.rgb, s.belt.w * side * low * 0.55);
    c = mix(c, s.belt.rgb, s.belt.w * (1.0 - side) * low * 0.4);
    // The glow round the disc, in rings.
    let r = length((p - s.sun.xy) * vec2<f32>(1.0, 1.15)) / (half.y * 0.9);
    let g = exp(-r * 3.0) * s.glow.w;
    let gq = clamp(floor(g * 7.0 + 0.25 + 0.5 * d) / 7.0, 0.0, 1.0);
    c = mix(c, s.glow.rgb, gq);
    // The aurora, glowing over it.
    if (s.aurora.x > 0.01) {
        let x = p.x - s.view.x;
        let near = curtain(x, p.y, 0.0);
        let far = curtain(x, p.y, 1.0);
        // (Bright: the night's light dims what's behind the world.)
        c = c + (near.rgb * near.a + far.rgb * far.a * (1.0 - near.a)) * s.aurora.x * 1.8;
    }
    return vec4<f32>(to_linear(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0))), s.zenith.w);
}
