// Open water mirroring the sky: the backdrop (the sky, the ranges, the
// clouds; drawn into an image by its own camera) flipped about each
// column's water surface, wobbling with ripples, fading with depth, darker
// in its ripples' lines. Only where the light grid found water open to the
// sky (`WaterMask`: per texel, how far below its surface). Drawn over the
// water, under what's in it; the light overlay grades it after.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Reflect {
    // The grid's area: left x, bottom y, width, height (world).
    grid: vec4<f32>,
    // The mask's texels across, down; cells a texel; the time (s).
    size: vec4<f32>,
    // The backdrop image's area: left x, bottom y, width, height (world).
    view: vec4<f32>,
    // x how strongly it mirrors (0: not at all).
    strength: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> r: Reflect;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var mask: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var mask_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var backdrop: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var backdrop_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xy;
    let msize = vec2<f32>(textureDimensions(mask));
    let tc = vec2<i32>(clamp(floor(in.uv * msize), vec2<f32>(0.0), msize - 1.0));
    let m = textureLoad(mask, tc, 0);
    let below = round(m.g * 255.0);
    if (m.r < 0.5 || below > 254.0) {
        discard;
    }
    let texel = r.size.z;
    // This texel's top (row 0 the grid's top), its column's surface above.
    let top = r.grid.y + r.grid.w - f32(tc.y) * texel;
    let surface = top + below * texel;
    let d = max(surface - p.y, 0.0);
    let t = r.size.w;
    // Ripples: the deeper, the more it wobbles.
    let wob = sin(t * 1.6 + p.y * 0.9) * min(d * 0.08, 1.6) + sin(t * 0.9 + p.x * 0.21 + p.y * 0.4) * min(d * 0.04, 0.8);
    let q = vec2<f32>(p.x + wob, surface + d);
    let uv = vec2<f32>((q.x - r.view.x) / r.view.z, 1.0 - (q.y - r.view.y) / r.view.w);
    if (uv.y < 0.0) {
        discard;
    }
    let c = textureSampleLevel(backdrop, backdrop_sampler, clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0);
    // (Its ripples' lines a little darker, every third cell down.)
    let line = select(1.0, 0.82, d > 2.0 && (u32(floor(d)) % 3u) == 0u);
    let a = r.strength.x * (1.0 - smoothstep(0.0, 56.0, d)) * c.a;
    return vec4<f32>(c.rgb * 0.8 * line, a);
}
