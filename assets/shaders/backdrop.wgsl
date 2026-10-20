// The surface's ranges, lit by the hour (crates/backdrop terrain.rs: what a
// texel holds; `terrain::shade` does the same on the CPU for stills). A
// texel: its face's normal (x, y; z toward the viewer follows), its
// material, a crest or edge, a rock's stroke, how high up the range. The
// light: toward the sun or moon, its colour, the ambient, how bright a
// crest facing it lights up; the sky's colour to fade into. In the looks'
// colours (sRGB), out linear; the light overlay grades it after.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Range {
    // xyz toward the light; w how bright a crest catches it.
    to: vec4<f32>,
    // rgb the light; w the tile's alpha.
    color: vec4<f32>,
    // rgb the ambient; w the layer's share of the haze.
    ambient: vec4<f32>,
    // rgb what distance fades into (the sky); w the mist at its feet.
    haze: vec4<f32>,
    // rgb rock; w the feet's row (a share of the strip from its top).
    rock: vec4<f32>,
    snow: vec4<f32>,
    // rgb scrub; w the tile's first column, mod 4 (the dither's phase).
    scrub: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> r: Range;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var data: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var data_sampler: sampler;

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
    let size = vec2<f32>(textureDimensions(data));
    let tc = vec2<i32>(clamp(floor(in.uv * size), vec2<f32>(0.0), size - 1.0));
    let p = textureLoad(data, tc, 0);
    if (p.a < 0.5) {
        discard;
    }
    let raw = vec4<u32>(round(p * 255.0));
    let nx = f32(raw.x) / 255.0 * 2.0 - 1.0;
    let ny = f32(raw.y) / 255.0 * 2.0 - 1.0;
    let n = vec3<f32>(nx, ny, -sqrt(max(0.0, 1.0 - nx * nx - ny * ny)));
    let bits = raw.z;
    let mat = bits & 3u;
    let crest = (bits & 4u) != 0u;
    let stroke = (bits & 8u) != 0u;
    let up = f32(bits >> 4u) / 15.0;
    // The light on the face, stepped into a few flat tones, dithered only
    // where one meets the next.
    let d = bayer(u32(tc.x) + u32(r.scrub.w), u32(tc.y));
    let facing = max(dot(n, r.to.xyz), 0.0);
    let lam = min(floor(facing * 5.0 + 0.25 + 0.5 * d) / 5.0, 1.0);
    var albedo = r.rock.rgb;
    if (mat == 1u) {
        albedo = r.snow.rgb;
    } else if (mat == 2u) {
        albedo = r.scrub.rgb;
    }
    // (The lower slopes darker: the light grazes up high.)
    let low = 0.62 + 0.38 * up;
    var c = albedo * (r.ambient.rgb + r.color.rgb * lam * 0.95) * low;
    if (crest && facing > 0.3) {
        c = c * (1.0 - 0.35 * r.to.w) + r.color.rgb * 0.75 * r.to.w + vec3<f32>(0.02);
    }
    if (stroke) {
        c = c * 0.55;
    }
    let h = size.y;
    let foot = r.rock.w * h;
    let mist = smoothstep(foot - h * 0.15, foot + h * 0.1, f32(tc.y));
    c = mix(c, r.haze.rgb, r.ambient.w + (1.0 - r.ambient.w) * r.haze.w * mist * 0.7);
    return vec4<f32>(to_linear(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0))), r.color.w);
}
