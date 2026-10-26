// The distortion of a channelled spell (game/src/magic/warp.rs). Round: a
// gravity well swirls and pinches the picture inside its reach, darkens its
// heart, rings it with a faint bright edge. A cone (force): wavefronts racing
// out along it (a push) or in (a pull), bending the picture along them and
// brightest at the fronts. Strength 0: the picture as it is.
//
// Then fields (game/src/magic/field.rs, warp.rs `FieldWarp`), over the
// well's picture: bubbles and columns of bent space, no borders, each fading
// to nothing toward its edge.
// - Repulsor: glass. It magnifies what's behind it, most in the middle, a
//   slow shimmer over it, the colours parting a little toward its edge;
//   rings race across it where it's struck.
// - Slow: the picture wavers like heat haze, smeared, greyed toward violet.
// - Updraft: the picture streams upward in ripples.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct FieldWarp {
    time: f32,
    aspect: f32,
    scale: f32,
    count: f32,
    shapes: array<vec4<f32>, 16>,
    ripples: array<vec4<f32>, 8>,
}

struct Warp {
    center: vec2<f32>,
    radius: f32,
    strength: f32,
    aspect: f32,
    time: f32,
    push: f32,
    cone: f32,
    dir: vec2<f32>,
    fields: FieldWarp,
}
@group(0) @binding(2) var<uniform> warp: Warp;

// The picture at `uv`, bent by the well.
fn well(uv: vec2<f32>) -> vec4<f32> {
    if (warp.strength <= 0.0) {
        return textureSampleLevel(screen_texture, texture_sampler, uv, 0.0);
    }
    // In units of the screen's height, so it's round.
    var d = uv - warp.center;
    d.x = d.x * warp.aspect;
    let r = length(d) / warp.radius;
    if (r >= 1.0) {
        return textureSampleLevel(screen_texture, texture_sampler, uv, 0.0);
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
            return textureSampleLevel(screen_texture, texture_sampler, uv, 0.0);
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
        let r_ = textureSampleLevel(screen_texture, texture_sampler, warp.center + qr, 0.0).r;
        let g_ = textureSampleLevel(screen_texture, texture_sampler, warp.center + qg, 0.0).g;
        let b_ = textureSampleLevel(screen_texture, texture_sampler, warp.center + qb, 0.0).b;
        let a_ = textureSampleLevel(screen_texture, texture_sampler, warp.center + qg, 0.0).a;
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
    var color = textureSampleLevel(screen_texture, texture_sampler, warp.center + q, 0.0);
    color = vec4<f32>(color.rgb * (1.0 - heart) + tint * ring, color.a);
    return color;
}

fn sample(uv: vec2<f32>) -> vec4<f32> {
    return well(clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var fw = warp.fields;
    if (fw.count <= 0.0) {
        return well(in.uv);
    }
    let t = fw.time;
    // What the fields here do: bend where the picture's drawn from, haze
    // it (slow), part its colours, and the ripples' brightness.
    var bend = vec2<f32>(0.0);
    var haze = 0.0;
    var tint = 0.0;
    var split = 0.0;
    var glass = 0.0;
    var light = vec3<f32>(0.0);
    for (var i = 0; i < 8; i++) {
        if (f32(i) >= fw.count) {
            break;
        }
        let a = fw.shapes[2 * i];
        let b = fw.shapes[2 * i + 1];
        let s = b.y;
        if (s <= 0.0 || a.z <= 0.0) {
            continue;
        }
        // (In screen heights from its middle or foot, so it's round; and in
        // cells, for patterns.)
        var d = in.uv - a.xy;
        d.x = d.x * fw.aspect;
        let w = vec2<f32>(d.x, -d.y) * fw.scale + vec2<f32>(b.z * 97.0, b.z * 61.0);
        if (a.w < 2.5) {
            let r = length(d) / a.z;
            if (r >= 1.0) {
                continue;
            }
            let body = 1.0 - r * r;
            if (a.w < 1.5) {
                // Repulsor: a lens, drawing from nearer its middle, slow
                // waves of force pulsing out through it.
                let pulse = 1.0 + 0.25 * sin(r * 9.0 - t * 4.0 + b.z * 6.28);
                let lens = 0.28 * s * body * pulse;
                var pull = -d * lens;
                let shimmer = vec2<f32>(sin(w.x * 0.21 + t * 1.7) * sin(w.y * 0.17 - t * 1.3), cos(w.x * 0.15 - t * 1.1) * sin(w.y * 0.23 + t * 1.5)) * 0.007 * s * body;
                pull = pull + shimmer;
                pull.x = pull.x / fw.aspect;
                bend += pull;
                split += 0.0025 * s * r * body;
                glass = max(glass, s * body);
                // (A faint glow, brightest in the middle, nothing at its
                // edge, breathing with the waves.)
                light += vec3<f32>(0.45, 0.7, 1.0) * s * body * (0.09 + 0.05 * sin(r * 14.0 - t * 4.0 + b.z * 6.28));
            } else {
                // Slow: a band of haze near its outside, fading both ways
                // (no line), slow ripples rolling out through it; the
                // middle clear, only faintly greyed, so what crawls there
                // shows.
                let band = s * smoothstep(0.35, 0.78, r) * smoothstep(1.0, 0.86, r);
                var waver = vec2<f32>(sin(w.y * 0.19 + t * 1.1) + 0.5 * sin(w.y * 0.47 - t * 0.6), cos(w.x * 0.17 - t * 0.9) + 0.4 * sin(w.x * 0.41 + t * 0.5)) * 0.008 * band;
                var roll = d / max(length(d), 1e-4) * sin(r * 34.0 - t * 1.6 + b.z * 6.28) * 0.006 * band;
                waver = waver + roll;
                waver.x = waver.x / fw.aspect;
                bend += waver;
                haze = max(haze, band);
                tint = max(tint, max(band * 0.75, s * 0.28 * smoothstep(1.0, 0.6, r)));
            }
        } else {
            // Updraft: a column from its foot (`a.y`) up to its top (`b.x`).
            let across = abs(d.x) / a.z;
            let up = (a.y - in.uv.y) / max(a.y - b.x, 1e-4);
            if (across >= 1.0 || up < -0.02 || up > 1.0) {
                continue;
            }
            let ends = smoothstep(-0.02, 0.06, up) * smoothstep(1.0, 0.85, up);
            let fall = s * smoothstep(1.0, 0.15, across) * ends;
            let rise = sin(w.y * 0.3 - t * 9.0 + w.x * 0.12) * 0.0038 + sin(w.y * 0.11 - t * 4.0) * 0.0022 + sin(w.y * 0.7 - t * 15.0 + w.x * 0.5) * 0.0012;
            let sway = sin(w.y * 0.17 - t * 5.0) * 0.0018;
            bend += vec2<f32>(sway / fw.aspect, rise) * fall;
            // (Pale streaks rising in it, soft.)
            let lane = floor(w.x / 3.0);
            let streak = pow(0.5 + 0.5 * sin(w.y * 0.35 - t * 12.0 + fract(sin(lane * 12.9898) * 43758.5) * 6.28), 12.0);
            light += vec3<f32>(0.5, 1.0, 0.9) * streak * 0.07 * fall;
        }
    }
    // Rings racing out from blows, across the glass.
    var ring = 0.0;
    if (glass > 0.0) {
        for (var i = 0; i < 8; i++) {
            let r = fw.ripples[i];
            if (r.w <= 0.0) {
                continue;
            }
            var d = in.uv - r.xy;
            d.x = d.x * fw.aspect;
            let dist = length(d);
            let fade = (1.0 - r.z / 0.8) * r.w;
            let wave = exp(-pow((dist - r.z * 0.3) * 60.0, 2.0)) * fade * glass;
            ring += wave;
            var out = d / max(dist, 1e-4);
            out.x = out.x / fw.aspect;
            bend += out * wave * 0.018;
            split += wave * 0.006;
        }
    }
    if (bend.x == 0.0 && bend.y == 0.0 && haze <= 0.0 && tint <= 0.0 && light.b <= 0.0) {
        return well(in.uv);
    }
    let uv = in.uv + bend;
    var c: vec3<f32>;
    if (haze > 0.02) {
        // Smeared along a slowly turning way.
        let a = t * 0.35;
        let dir = vec2<f32>(cos(a), sin(a)) * 0.0035 * haze;
        c = vec3<f32>(0.0);
        for (var k = -2; k <= 2; k++) {
            c += sample(uv + dir * f32(k)).rgb;
        }
        c = c / 5.0;
    } else if (split > 0.0) {
        c = vec3<f32>(sample(uv + vec2<f32>(split, 0.0)).r, sample(uv).g, sample(uv - vec2<f32>(split, 0.0)).b);
    } else {
        c = sample(uv).rgb;
    }
    // (Greyed toward violet, as time thickens.)
    if (tint > 0.0) {
        let l = dot(c, vec3<f32>(0.3, 0.59, 0.11));
        c = mix(c, vec3<f32>(l) * vec3<f32>(0.84, 0.7, 1.15), tint) + vec3<f32>(0.3, 0.16, 0.55) * 0.12 * tint;
    }
    c = c + vec3<f32>(0.55, 0.78, 1.0) * ring * 0.35 + light;
    return vec4<f32>(c, sample(in.uv).a);
}
