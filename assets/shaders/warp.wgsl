// The distortion of a channelled spell (game/src/magic/warp.rs). Round: a
// gravity well swirls and pinches the picture inside its reach, darkens its
// heart, rings it with a faint bright edge. A cone (force): wavefronts racing
// out along it (a push) or in (a pull), bending the picture along them and
// brightest at the fronts. Strength 0: the picture as it is.

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
    cone: f32,
    dir: vec2<f32>,
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
    var tint = vec3<f32>(0.75, 0.7, 1.0);
    if (warp.cone > 0.0) {
        // Force: only inside the cone, softening toward its edges and ends.
        let along = dot(normalize(d), normalize(warp.dir));
        let angle = acos(clamp(along, -1.0, 1.0));
        let inside = smoothstep(warp.cone, warp.cone * 0.55, angle) * smoothstep(0.0, 0.05, r) * (1.0 - r * r);
        if (inside <= 0.0) {
            return textureSample(screen_texture, texture_sampler, in.uv);
        }
        // Sharp wavefronts, racing out (a push) or in (a pull).
        let travel = select(warp.time * 11.0, -warp.time * 9.0, warp.push > 0.5);
        let wave = pow(0.5 + 0.5 * sin(r * 16.0 - travel), 5.0);
        // Bent along them: a push stretches the picture outward at each
        // front (drawn from nearer in), a pull squeezes it; and the whole
        // cone shivers a little across its width.
        let s = select(1.0, -1.0, warp.push > 0.5);
        let side = vec2<f32>(-d.y, d.x) * sin(warp.time * 23.0 + r * 40.0) * 0.012;
        q = d * (1.0 + s * warp.strength * inside * (0.04 + 0.16 * wave)) + side * warp.strength * inside;
        // The fronts split the colours a little (red and blue pulled apart),
        // and a faint haze fills the cone, brightest at the hand.
        let split = 0.018 * wave * inside * warp.strength;
        var qr = q * (1.0 + split);
        var qb = q * (1.0 - split);
        qr.x = qr.x / warp.aspect;
        qb.x = qb.x / warp.aspect;
        var qg = q;
        qg.x = qg.x / warp.aspect;
        let r_ = textureSample(screen_texture, texture_sampler, warp.center + qr).r;
        let g_ = textureSample(screen_texture, texture_sampler, warp.center + qg).g;
        let b_ = textureSample(screen_texture, texture_sampler, warp.center + qb).b;
        let a_ = textureSample(screen_texture, texture_sampler, warp.center + qg).a;
        let glow = wave * inside * 0.9 * warp.strength + inside * (0.22 - 0.12 * r) * warp.strength;
        return vec4<f32>(vec3<f32>(r_, g_, b_) + vec3<f32>(0.55, 0.78, 1.0) * glow, a_);
    } else if (warp.push > 0.5) {
        // A push all round: the middle swells, rippling outward.
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
    color = vec4<f32>(color.rgb * (1.0 - heart) + tint * ring, color.a);
    return color;
}
