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
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> s: Sky;

fn bayer(x: u32, y: u32) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return (m[(y & 3u) * 4u + (x & 3u)] + 0.5) / 16.0;
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
    return vec4<f32>(to_linear(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0))), s.zenith.w);
}
