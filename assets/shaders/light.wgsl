// Lighting overlay (SPEC §4.1). The texture holds light per texel, stored as
// sqrt(light) for precision in the dark; sampled smoothly, squared back, and
// dithered so dark gradients don't band. The pipeline's blend decides what
// it does: multiply over the scene (light) or add (glow haze).
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var light_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var light_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let s = textureSample(light_texture, light_sampler, in.uv).rgb;
    var l = s * s;
    // Ordered dither, a quarter of an 8-bit step.
    let p = vec2<u32>(in.position.xy);
    let bayer = f32(((p.x ^ p.y) & 1u) * 2u + (p.y & 1u)) / 4.0 - 0.375;
    l = max(l + vec3<f32>(bayer / 255.0), vec3<f32>(0.0));
    return vec4<f32>(l, 1.0);
}
