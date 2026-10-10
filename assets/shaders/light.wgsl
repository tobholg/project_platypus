// Lighting overlay (SPEC §4.1). The texture holds light per texel, stored as
// sqrt(light) for precision in the dark; sampled smoothly (a cubic B-spline
// over the texels, from four bilinear taps: plain bilinear left a texel's
// steps along a flashlight beam's edge), squared back, and dithered so dark
// gradients don't band. The pipeline's blend decides what it does: multiply
// over the scene (light) or add (glow haze).
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var light_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var light_sampler: sampler;

// A cubic B-spline sample: four bilinear taps, each between two texels,
// weighted so together they make the 4x4 B-spline.
fn smooth_sample(uv: vec2<f32>) -> vec3<f32> {
    let size = vec2<f32>(textureDimensions(light_texture));
    let p = uv * size - 0.5;
    let i = floor(p);
    let f = p - i;
    let f2 = f * f;
    let f3 = f2 * f;
    let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
    let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
    let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
    let w3 = f3 / 6.0;
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    let h0 = w1 / g0 - 1.0;
    let h1 = w3 / g1 + 1.0;
    let t0 = (i + vec2<f32>(h0.x, h0.y) + 0.5) / size;
    let t1 = (i + vec2<f32>(h1.x, h0.y) + 0.5) / size;
    let t2 = (i + vec2<f32>(h0.x, h1.y) + 0.5) / size;
    let t3 = (i + vec2<f32>(h1.x, h1.y) + 0.5) / size;
    let a = textureSample(light_texture, light_sampler, t0).rgb * g0.x + textureSample(light_texture, light_sampler, t1).rgb * g1.x;
    let b = textureSample(light_texture, light_sampler, t2).rgb * g0.x + textureSample(light_texture, light_sampler, t3).rgb * g1.x;
    return a * g0.y + b * g1.y;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let s = smooth_sample(in.uv);
    var l = s * s;
    // Ordered dither, a quarter of an 8-bit step.
    let p = vec2<u32>(in.position.xy);
    let bayer = f32(((p.x ^ p.y) & 1u) * 2u + (p.y & 1u)) / 4.0 - 0.375;
    l = max(l + vec3<f32>(bayer / 255.0), vec3<f32>(0.0));
    return vec4<f32>(l, 1.0);
}
