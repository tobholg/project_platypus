// The far clouds, lit by the hour (crates/backdrop peaks.rs `cloud_data`:
// what a texel holds; `cloud_shade` does the same on the CPU). A texel:
// its puff's face (x, y; z toward the viewer follows), how deep into the
// heap, on its outline or not; its alpha. Lit toward the sun or moon in a
// few steps; the shade the sky's; deeper darker, unless the sun's low:
// then the underside glows; the outline facing the light lined with it.
// Each pixel blends its four nearest texels, each lit first: the clouds
// drift smoothly. In sRGB, out linear.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Cloud {
    // xyz toward the light; w how golden the hour.
    to: vec4<f32>,
    // rgb the light; w the tile's alpha.
    color: vec4<f32>,
    ambient: vec4<f32>,
    haze: vec4<f32>,
    // the clouds' lit and shaded colours.
    lit: vec4<f32>,
    shade: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> c: Cloud;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var data: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var data_sampler: sampler;

fn bayer(x: u32, y: u32) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return (m[(y & 3u) * 4u + (x & 3u)] + 0.5) / 16.0;
}

fn to_linear(v: vec3<f32>) -> vec3<f32> {
    let lo = v / 12.92;
    let hi = pow((v + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, v <= vec3<f32>(0.04045));
}

// A texel lit, premultiplied (rgb × alpha, alpha).
fn shade(t: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(data));
    let q = clamp(t, vec2<i32>(0), size - 1);
    let p = textureLoad(data, q, 0);
    if (p.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let raw = vec3<u32>(round(p.rgb * 255.0));
    let nx = f32(raw.x) / 255.0 * 2.0 - 1.0;
    let ny = f32(raw.y) / 255.0 * 2.0 - 1.0;
    let n = vec3<f32>(nx, ny, -sqrt(max(0.0, 1.0 - nx * nx - ny * ny)));
    let depth = f32(raw.z & 63u) / 63.0;
    let edge = (raw.z & 64u) != 0u;
    let golden = c.to.w;
    let facing = max(dot(n, c.to.xyz), 0.0);
    let lam = min(floor(facing * 4.0 + 0.25 + 0.5 * bayer(u32(q.x), u32(q.y))) / 4.0, 1.0);
    let lit_c = c.lit.rgb * (c.ambient.rgb * 0.5 + c.color.rgb * 0.75);
    // (A cloud's shade is bright still: light scatters through it.)
    let shade_c = c.shade.rgb * (c.ambient.rgb * 0.6 + vec3<f32>(0.45));
    var v = mix(shade_c, lit_c, lam);
    v = v * (1.0 - 0.3 * depth * (1.0 - golden));
    v = mix(v, vec3<f32>(1.0, 0.6, 0.45) * c.color.r, golden * depth * 0.45);
    if (edge && facing > 0.2) {
        v = v * 0.5 + c.color.rgb * 0.62 + vec3<f32>(0.12);
    }
    v = mix(v, c.haze.rgb, 0.12);
    return vec4<f32>(v * p.a, p.a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(data));
    let p = in.uv * size - 0.5;
    let i = vec2<i32>(floor(p));
    let f = p - floor(p);
    let a = mix(shade(i), shade(i + vec2<i32>(1, 0)), f.x);
    let b = mix(shade(i + vec2<i32>(0, 1)), shade(i + vec2<i32>(1, 1)), f.x);
    let m = mix(a, b, f.y);
    if (m.a <= 0.001) {
        discard;
    }
    let rgb = clamp(m.rgb / m.a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(to_linear(rgb), m.a * c.color.w);
}
