// The distortion around a gravity well (game/src/magic/warp.rs): swirl and
// pinch the picture inside its reach, darken its heart, ring it with a faint
// bright edge. Strength 0: the picture as it is.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct Warp {
    center: vec2<f32>,
    radius: f32,
    strength: f32,
    aspect: f32,
    time: f32,
    push: f32,
    _pad: f32,
}
@group(0) @binding(2) var<uniform> warp: Warp;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if (warp.strength <= 0.0) {
        return textureSample(screen_texture, texture_sampler, in.uv);
    }
    // In units of the screen's height, so it's round.
    var d = in.uv - warp.center;
    d.x = d.x * warp.aspect;
    let r = length(d) / warp.radius;
    if (r >= 1.0) {
        return textureSample(screen_texture, texture_sampler, in.uv);
    }
    let fall = (1.0 - r) * (1.0 - r);
    var q = d;
    var heart = 0.0;
    var ring = 0.0;
    if (warp.push > 0.5) {
        // A push: the middle swells (drawn from nearer in), rippling outward.
        let wave = sin(r * 22.0 - warp.time * 14.0) * 0.5 + 0.5;
        q = q * (1.0 + 0.4 * warp.strength * fall) * (1.0 - 0.03 * warp.strength * wave * (1.0 - r));
        ring = wave * (1.0 - r) * 0.12 * warp.strength;
    } else {
        // Drawing in: swirl (more toward the middle, breathing a little) and
        // pinch (the middle draws from further out, so the ring bulges).
        let breathe = 1.0 + 0.12 * sin(warp.time * 3.1 + r * 9.0);
        let angle = warp.strength * 3.2 * fall * breathe;
        let c = cos(angle);
        let s = sin(angle);
        q = vec2<f32>(d.x * c - d.y * s, d.x * s + d.y * c);
        q = q * (1.0 - 0.45 * warp.strength * fall);
        // A dark heart and a faint bright ring near the edge of the pull.
        heart = smoothstep(0.32, 0.0, r) * 0.55 * warp.strength;
        ring = exp(-pow((r - 0.78) * 14.0, 2.0)) * 0.22 * warp.strength;
    }
    q.x = q.x / warp.aspect;
    var color = textureSample(screen_texture, texture_sampler, warp.center + q);
    color = vec4<f32>(color.rgb * (1.0 - heart) + vec3<f32>(0.75, 0.7, 1.0) * ring, color.a);
    return color;
}
