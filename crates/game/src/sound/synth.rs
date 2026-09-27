//! Sounds made from recipes (`make` in sounds.ron): every one rendered here,
//! offline, to a stereo buffer the audio engine then just plays. The kit
//! is small: seeded noise, oscillators, fundsp's state-variable filters
//! (swept per sample) and its reverb, envelopes, echo. Each recipe is a
//! function of its parameters and a seed (a variant).

use fundsp::prelude32::{AudioUnit, bandpass, highpass, lowpass, reverb_stereo};
use serde::Deserialize;
use std::f32::consts::{PI, TAU};

/// What a sound is made of (`make:` in sounds.ron).
#[derive(Clone, Debug, Deserialize)]
pub enum Make {
    /// A body blow: a low tone falling in pitch, a burst of noise.
    Thud { pitch: f32, #[serde(default = "half")] drop: f32, #[serde(default = "half")] noise: f32, length: f32, #[serde(default = "bright")] bright: f32 },
    /// Struck metal or stone: a few inharmonic partials ringing down, a
    /// gritty burst with them (`grit`: stone's scrape rather than a bell),
    /// muffled above `muffle` Hz.
    Clang { pitch: f32, ring: f32, length: f32, #[serde(default)] grit: f32, #[serde(default = "open")] muffle: f32 },
    /// Air moved: noise through a band sweeping from `from` to `to` (Hz).
    Whoosh { from: f32, to: f32, length: f32, #[serde(default = "one")] q: f32 },
    /// A step, a scrape, a knock: a short filtered noise burst, with a thump
    /// under it (`body` Hz, 0: none) and grit (crunch: gravel, snow).
    Burst { cutoff: f32, length: f32, #[serde(default = "one")] q: f32, #[serde(default)] body: f32, #[serde(default)] grit: f32 },
    /// A water drop: a tone rising fast ("plink").
    Drip { pitch: f32, #[serde(default = "half")] rise: f32, length: f32 },
    /// A blast: a crack falling to a rumble, and a sub drop.
    Boom { length: f32, #[serde(default = "one")] sub: f32 },
    /// Electric: a buzz jumping in pitch, crackling.
    Zap { pitch: f32, length: f32 },
    /// Bell-like notes (semitones above `pitch`), one after another.
    Chime { pitch: f32, notes: Vec<f32>, length: f32 },
    /// A sweep between two pitches (UI, pickups).
    Blip { pitch: f32, to: f32, length: f32 },
    /// A footstep: a soft pat, noise about `scuff` Hz over a little low
    /// weight (`heel` Hz, 0: none), a touch of crunch (`grit`), all
    /// muffled above `muffle` Hz.
    Step { #[serde(default)] heel: f32, scuff: f32, #[serde(default = "one")] q: f32, #[serde(default)] grit: f32, length: f32, #[serde(default = "muffle_step")] muffle: f32 },
    /// A swing through the air: noise through a band that rises to `high`
    /// as the swing peaks and falls back toward `low` (the blade passing),
    /// a thin whistle over it (`whistle`: a blade's edge).
    Swish { low: f32, high: f32, length: f32, #[serde(default = "q_swish")] q: f32, #[serde(default)] whistle: f32 },
    /// A blow landing, in layers: a sharp crack, a punch (`body` Hz,
    /// falling), a wet thwack (noise about `flesh` Hz, falling), a blade's
    /// slice (high hiss), and a glassy ring (being hurt).
    /// All of it muffled above `muffle` Hz.
    Impact { body: f32, #[serde(default = "half")] crack: f32, flesh: f32, #[serde(default)] slice: f32, #[serde(default)] ring: f32, length: f32, #[serde(default = "open")] muffle: f32 },
    /// A blade biting into a body, in layers a few milliseconds apart: the
    /// edge's bite (a bright tick sweeping down: `edge`), the chop (noise
    /// about `chop` Hz, falling, with a wet flutter: `wet`), the weight
    /// behind it (`weight` Hz, dropping from about three times that), and
    /// a low rumble under all; `tear`: tiny wet bursts a second, scattered
    /// through it (flesh parting); saturated a little, muffled above
    /// `muffle`. `weight` 0: no thump at all. `slice`: a longer cut over
    /// the bite (noise sweeping down from high, the blade going through);
    /// `soft`: the chop's rise (s: longer, less of a knock at the start).
    Slash { chop: f32, weight: f32, #[serde(default = "half")] edge: f32, #[serde(default = "half")] wet: f32, #[serde(default)] tear: f32, #[serde(default)] slice: f32, #[serde(default = "chop_rise")] soft: f32, length: f32, #[serde(default = "open")] muffle: f32 },
    /// A blade's cut, shaped as its swing: noise through a band sweeping
    /// up from `low` to `high` Hz as the blade arrives (`length` s), cut
    /// off sharply at the contact with a bright snap, then a short fall
    /// (the band dropping, `tail` s) with tiny wet bursts (`tear` a
    /// second); nothing under 500 Hz. `meat`: a chunk of body at the
    /// contact (noise about 900 Hz; the floor lowers towards 300 Hz);
    /// `crunch`: dense gritty bursts and a little drive.
    Cut { low: f32, high: f32, length: f32, #[serde(default = "cut_tail")] tail: f32, #[serde(default)] tear: f32, #[serde(default = "half")] snap: f32, #[serde(default)] meat: f32, #[serde(default)] crunch: f32 },
    /// Loops (`length` s): a fire's roar, hiss and crackles (per second).
    Fire { crackle: f32, roar: f32, #[serde(default = "tenth")] hiss: f32 },
    /// A cave: a low rumble, air moving through it, a faint low hum.
    Cave { rumble: f32, wind: f32, #[serde(default)] hum: f32 },
    /// Rain: a steady hiss and drops (per second).
    Rain { drops: f32 },
    /// Wind in the open, in gusts.
    Wind { gust: f32 },
    /// Moving water: babble and bubbles (per second).
    Water { bubbles: f32 },
    /// Lava: a low churn and glops (per second).
    Lava { glops: f32 },
    /// A rocket's thrust: a roar, a hiss, fluttering (`flutter` Hz).
    Thrust { roar: f32, hiss: f32, flutter: f32 },
    /// Music: pads through a chord progression (semitones from `root` Hz),
    /// `chord` s each, a filter at `bright` Hz, and plucked notes from
    /// `scale` (the chance a beat has one).
    Pads { root: f32, chords: Vec<Vec<f32>>, chord: f32, bright: f32, #[serde(default)] plucks: f32, #[serde(default)] scale: Vec<f32> },
}

fn half() -> f32 {
    0.5
}
fn one() -> f32 {
    1.0
}
fn tenth() -> f32 {
    0.1
}
fn bright() -> f32 {
    2500.0
}
fn q_swish() -> f32 {
    1.6
}
fn muffle_step() -> f32 {
    1500.0
}
fn open() -> f32 {
    18_000.0
}

/// A stereo buffer.
#[derive(Clone, Debug, Default)]
pub struct Buf {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Buf {
    fn mono(v: Vec<f32>) -> Self {
        Buf { r: v.clone(), l: v }
    }

    fn zeros(n: usize) -> Self {
        Buf { l: vec![0.0; n], r: vec![0.0; n] }
    }

    pub fn len(&self) -> usize {
        self.l.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    pub fn peak(&self) -> f32 {
        self.l.iter().chain(&self.r).fold(0.0f32, |m, x| m.max(x.abs()))
    }

    pub fn rms(&self) -> f32 {
        let n = (self.l.len() * 2).max(1) as f32;
        (self.l.iter().chain(&self.r).map(|x| x * x).sum::<f32>() / n).sqrt()
    }

    fn scale(&mut self, k: f32) {
        for x in self.l.iter_mut().chain(self.r.iter_mut()) {
            *x *= k;
        }
    }

    /// Add `x` at frame `i`, panned (-1 left .. 1 right).
    fn add(&mut self, i: usize, x: f32, pan: f32) {
        if i < self.l.len() {
            let a = (pan.clamp(-1.0, 1.0) + 1.0) * PI / 4.0;
            self.l[i] += x * a.cos() * std::f32::consts::SQRT_2;
            self.r[i] += x * a.sin() * std::f32::consts::SQRT_2;
        }
    }
}

/// A seeded random stream (xorshift*): variants differ, reruns don't.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0..1.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// -1..1.
    fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
}

#[derive(Clone, Copy)]
enum Pass {
    Low,
    Band,
    High,
}

/// A state-variable filter whose cutoff can move every sample.
struct Filter {
    unit: Box<dyn AudioUnit>,
    q: f32,
    nyquist: f32,
}

impl Filter {
    fn new(pass: Pass, q: f32, sr: f32) -> Self {
        let mut unit: Box<dyn AudioUnit> = match pass {
            Pass::Low => Box::new(lowpass()),
            Pass::Band => Box::new(bandpass()),
            Pass::High => Box::new(highpass()),
        };
        unit.set_sample_rate(sr as f64);
        Filter { unit, q, nyquist: sr * 0.45 }
    }

    fn tick(&mut self, x: f32, hz: f32) -> f32 {
        let mut out = [0.0f32];
        self.unit.tick(&[x, hz.clamp(20.0, self.nyquist), self.q], &mut out);
        out[0]
    }
}

/// Brown(ish) noise: white, integrated and leaking back to zero, with the
/// drift under ~20 Hz taken out (no one hears it; it only ate headroom).
#[derive(Default)]
struct Brown {
    level: f32,
    last: f32,
    out: f32,
}

impl Brown {
    fn tick(&mut self, rng: &mut Rng) -> f32 {
        self.level = (self.level + rng.noise() * 0.06) * 0.997;
        self.out = self.level - self.last + 0.997 * self.out;
        self.last = self.level;
        self.out * 3.0
    }
}

/// A slow random wander in 0..1 (gusts, a fire's breathing), moving at
/// about `speed` a second.
struct Wander {
    at: f32,
    to: f32,
    speed: f32,
}

impl Wander {
    fn new(rng: &mut Rng, speed: f32) -> Self {
        Wander { at: rng.unit(), to: rng.unit(), speed }
    }

    fn tick(&mut self, rng: &mut Rng, dt: f32) -> f32 {
        let step = self.speed * dt;
        if (self.to - self.at).abs() <= step {
            self.to = rng.unit();
        }
        self.at += (self.to - self.at).signum() * step;
        // (Smoothed: eased toward, not straight lines.)
        self.at
    }
}

fn frames(secs: f32, sr: f32) -> usize {
    (secs.max(0.001) * sr) as usize
}

/// Attack then an exponential fall to about -40 dB over `decay` s.
fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack { t / attack.max(1e-4) } else { (-(t - attack) * 4.6 / decay.max(1e-4)).exp() }
}

/// Render `make` at `sr` Hz: `loops` s of a seamless loop (0: a one-shot,
/// its natural length).
pub fn render(make: &Make, sr: f32, seed: u64, loops: f32) -> Buf {
    let mut rng = Rng::new(seed);
    let rng = &mut rng;
    // Loops render a little more, to fold the end over the start.
    let fold = if loops > 0.0 { 2.0 } else { 0.0 };
    let total = loops + fold;
    match make {
        Make::Thud { pitch, drop, noise, length, bright } => Buf::mono(thud(*pitch, *drop, *noise, *length, *bright, sr, rng)),
        Make::Clang { pitch, ring, length, grit, muffle } => Buf::mono(clang(*pitch, *ring, *length, *grit, *muffle, sr, rng)),
        Make::Whoosh { from, to, length, q } => Buf::mono(whoosh(*from, *to, *length, *q, sr, rng)),
        Make::Burst { cutoff, length, q, body, grit } => Buf::mono(burst(*cutoff, *length, *q, *body, *grit, sr, rng)),
        Make::Drip { pitch, rise, length } => Buf::mono(drip(*pitch, *rise, *length, sr, rng)),
        Make::Boom { length, sub } => Buf::mono(boom(*length, *sub, sr, rng)),
        Make::Zap { pitch, length } => Buf::mono(zap(*pitch, *length, sr, rng)),
        Make::Chime { pitch, notes, length } => Buf::mono(chime(*pitch, notes, *length, sr, rng)),
        Make::Blip { pitch, to, length } => Buf::mono(blip(*pitch, *to, *length, sr)),
        Make::Step { heel, scuff, q, grit, length, muffle } => Buf::mono(step(*heel, *scuff, *q, *grit, *length, *muffle, sr, rng)),
        Make::Swish { low, high, length, q, whistle } => Buf::mono(swish(*low, *high, *length, *q, *whistle, sr, rng)),
        Make::Impact { body, crack, flesh, slice, ring, length, muffle } => Buf::mono(impact(*body, *crack, *flesh, *slice, *ring, *length, *muffle, sr, rng)),
        Make::Slash { chop, weight, edge, wet, tear, slice, soft, length, muffle } => Buf::mono(slash(&SlashParts { chop: *chop, weight: *weight, edge: *edge, wet: *wet, tear: *tear, slice: *slice, soft: *soft, length: *length, muffle: *muffle }, sr, rng)),
        Make::Cut { low, high, length, tail, tear, snap, meat, crunch } => Buf::mono(cut(&CutParts { low: *low, high: *high, length: *length, tail: *tail, tear: *tear, snap: *snap, meat: *meat, crunch: *crunch }, sr, rng)),
        Make::Fire { crackle, roar, hiss } => looped(fire(*crackle, *roar, *hiss, total, sr, rng), loops, sr),
        Make::Cave { rumble, wind, hum } => looped(cave(*rumble, *wind, *hum, total, sr, rng), loops, sr),
        Make::Rain { drops } => looped(rain(*drops, total, sr, rng), loops, sr),
        Make::Wind { gust } => looped(wind(*gust, total, sr, rng), loops, sr),
        Make::Water { bubbles } => looped(water(*bubbles, total, sr, rng), loops, sr),
        Make::Lava { glops } => looped(lava(*glops, total, sr, rng), loops, sr),
        Make::Thrust { roar, hiss, flutter } => looped(thrust(*roar, *hiss, *flutter, total, sr, rng), loops, sr),
        Make::Pads { root, chords, chord, bright, plucks, scale } => looped(pads(*root, chords, *chord, *bright, *plucks, scale, total, sr, rng), loops, sr),
    }
}

/// Fold the extra at the end over the start (an equal-power crossfade),
/// so the loop has no seam; `secs` long.
fn looped(mut b: Buf, secs: f32, sr: f32) -> Buf {
    let n = frames(secs, sr).min(b.len());
    let x = (b.len() - n).min(n);
    for ch in [&mut b.l, &mut b.r] {
        for i in 0..x {
            let k = i as f32 / x as f32;
            let (fade_in, fade_out) = ((k * PI / 2.0).sin(), (k * PI / 2.0).cos());
            ch[i] = ch[i] * fade_in + ch[n + i] * fade_out;
        }
        ch.truncate(n);
    }
    b
}

/// Echoes: `delay` s apart, each `feedback` of the last (the right a
/// little later than the left: width). Loops keep their length; one-shots
/// grow a tail.
pub fn echo(b: &mut Buf, delay: f32, feedback: f32, sr: f32, tail: bool) {
    if feedback <= 0.0 || delay <= 0.0 {
        return;
    }
    let grow = if tail { frames(delay * 6.0, sr) } else { 0 };
    for (ch, d) in [(&mut b.l, delay), (&mut b.r, delay * 1.13)] {
        let d = frames(d, sr);
        ch.resize(ch.len() + grow, 0.0);
        for i in d..ch.len() {
            let back = ch[i - d];
            ch[i] += back * feedback;
        }
    }
    let n = b.l.len().min(b.r.len());
    b.l.truncate(n);
    b.r.truncate(n);
}

/// Reverb (fundsp's), `wet` of it mixed in; one-shots grow its tail.
pub fn reverb(b: &mut Buf, wet: f32, room: f32, time: f32, sr: f32, tail: bool) {
    if wet <= 0.0 {
        return;
    }
    let mut unit: Box<dyn AudioUnit> = Box::new(reverb_stereo(room.clamp(1.0, 100.0), time.max(0.1), 0.5));
    unit.set_sample_rate(sr as f64);
    let grow = if tail { frames(time, sr) } else { 0 };
    b.l.resize(b.l.len() + grow, 0.0);
    b.r.resize(b.r.len() + grow, 0.0);
    // (A loop: its end run through first, so the room's already full as
    // it starts again.)
    if !tail {
        let mut out = [0.0f32; 2];
        for i in b.len().saturating_sub(frames(time * 2.0, sr))..b.len() {
            unit.tick(&[b.l[i], b.r[i]], &mut out);
        }
    }
    let mut out = [0.0f32; 2];
    for i in 0..b.len() {
        unit.tick(&[b.l[i], b.r[i]], &mut out);
        b.l[i] = b.l[i] * (1.0 - wet * 0.5) + out[0] * wet;
        b.r[i] = b.r[i] * (1.0 - wet * 0.5) + out[1] * wet;
    }
}

/// To a peak of `peak`, or (`rms` > 0) to that loudness, its rare peaks
/// (a loud crackle, a big drop) rounded off rather than the whole turned
/// down for them.
pub fn level(b: &mut Buf, peak: f32, rms: f32) {
    if rms > 0.0 {
        b.scale(rms / b.rms().max(1e-6));
        for x in b.l.iter_mut().chain(b.r.iter_mut()) {
            *x = 0.98 * (*x / 0.98).tanh();
        }
    } else {
        b.scale(peak / b.peak().max(1e-6));
    }
}

// ---- one-shots ----

fn thud(pitch: f32, drop: f32, noise: f32, length: f32, bright: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let mut f = Filter::new(Pass::Low, 0.7, sr);
    let mut ph = rng.unit();
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            ph += pitch * (1.0 + drop * 2.0 * (-t / 0.03).exp()) / sr;
            let body = (TAU * ph).sin() * env(t, 0.002, length);
            let click = f.tick(rng.noise(), bright) * env(t, 0.001, 0.05) * noise * 2.0;
            (1.6 * (body + click)).tanh()
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn clang(pitch: f32, ring: f32, length: f32, grit: f32, muffle: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let partials = [(1.0, 1.0), (2.76, 0.6), (5.40, 0.35), (8.93, 0.22), (13.34, 0.12)];
    let pitch = pitch * rng.range(0.97, 1.03);
    let phases: Vec<f32> = partials.iter().map(|_| rng.unit()).collect();
    let (mut hp, mut scrape, mut soft) = (Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Band, 1.5, sr), Filter::new(Pass::Low, 0.6, sr));
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let x = rng.noise();
            let rough = scrape.tick(x, pitch * 1.3) * env(t, 0.0008, 0.035) * grit * 3.0;
            let tone: f32 = partials
                .iter()
                .zip(&phases)
                .enumerate()
                .map(|(k, ((ratio, amp), ph))| amp * (TAU * (pitch * ratio * t + ph)).sin() * env(t, 0.0005, ring / (1.0 + k as f32 * 0.7)))
                .sum();
            let tick = hp.tick(x, 3000.0) * env(t, 0.0003, 0.02);
            soft.tick(tone * 0.6 + tick * 0.8 + rough, muffle)
        })
        .collect()
}

fn whoosh(from: f32, to: f32, length: f32, q: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let mut f = Filter::new(Pass::Band, q, sr);
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let hz = from * (to / from.max(1.0)).powf(k);
            f.tick(rng.noise(), hz) * (PI * k).sin().powf(1.5) * 2.5
        })
        .collect()
}

fn burst(cutoff: f32, length: f32, q: f32, body: f32, grit: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let mut f = Filter::new(Pass::Band, q, sr);
    let cutoff = cutoff * rng.range(0.85, 1.15);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let mut x = rng.noise();
            // Grit: now and then a sharp grain.
            if grit > 0.0 && rng.unit() < grit * 0.02 {
                x += rng.noise() * 6.0;
            }
            let hiss = f.tick(x, cutoff) * env(t, 0.001, length) * 2.0;
            ph += body / sr;
            let thump = if body > 0.0 { (TAU * ph).sin() * env(t, 0.001, 0.04) * 0.8 } else { 0.0 };
            hiss + thump
        })
        .collect()
}

fn drip(pitch: f32, rise: f32, length: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let pitch = pitch * rng.range(0.8, 1.25);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            ph += pitch * (1.0 + rise * (1.0 - (-t / 0.012).exp())) / sr;
            (TAU * ph).sin() * env(t, 0.0015, length)
        })
        .collect()
}

fn boom(length: f32, sub: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let mut f = Filter::new(Pass::Low, 0.8, sr);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let k = t / length;
            let crack = f.tick(rng.noise(), 6000.0 * (150.0f32 / 6000.0).powf((k / 0.6).min(1.0))) * env(t, 0.001, length * 0.8);
            ph += (32.0 + 55.0 * (-t / 0.15).exp()) / sr;
            let low = (TAU * ph).sin() * env(t, 0.004, length * 0.6) * sub;
            (1.8 * (crack * 1.5 + low)).tanh()
        })
        .collect()
}

fn zap(pitch: f32, length: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let mut f = Filter::new(Pass::Band, 0.8, sr);
    let (mut ph, mut hz, mut hold) = (0.0f32, pitch, 0usize);
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            if hold == 0 {
                hz = pitch * rng.range(0.5, 2.0);
                hold = frames(rng.range(0.006, 0.014), sr);
            }
            hold -= 1;
            ph = (ph + hz / sr).fract();
            let saw = ph * 2.0 - 1.0;
            let crackle = if rng.unit() < 0.01 { rng.noise() * 3.0 } else { 0.0 };
            f.tick(saw + crackle + rng.noise() * 0.3, 2200.0) * env(t, 0.001, length) * 2.0
        })
        .collect()
}

fn chime(pitch: f32, notes: &[f32], length: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length + notes.len() as f32 * 0.07, sr);
    let mut out = vec![0.0; n];
    for (k, semis) in notes.iter().enumerate() {
        let start = frames(k as f32 * 0.07, sr);
        let hz = pitch * 2f32.powf(semis / 12.0) * rng.range(0.998, 1.002);
        for (i, o) in out.iter_mut().enumerate().skip(start) {
            let t = (i - start) as f32 / sr;
            let tone = (TAU * hz * t).sin() + 0.3 * (TAU * hz * 2.0 * t).sin() + 0.1 * (TAU * hz * 3.01 * t).sin();
            *o += tone * env(t, 0.003, length) * 0.5;
        }
    }
    out
}

fn blip(pitch: f32, to: f32, length: f32, sr: f32) -> Vec<f32> {
    let n = frames(length, sr);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            ph += pitch * (to / pitch.max(1.0)).powf(k) / sr;
            let x = (TAU * ph).sin() + 0.25 * (3.0 * TAU * ph).sin();
            x * env(k * length, 0.002, length) * 0.8
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn step(heel: f32, scuff: f32, q: f32, grit: f32, length: f32, muffle: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length + 0.03, sr);
    let (mut bp, mut soft) = (Filter::new(Pass::Band, q, sr), Filter::new(Pass::Low, 0.6, sr));
    let scuff = scuff * rng.range(0.85, 1.15);
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            ph += heel / sr;
            let weight = if heel > 0.0 { (TAU * ph).sin() * env(t, 0.006, 0.04) * 0.15 } else { 0.0 };
            let mut x = rng.noise();
            if grit > 0.0 && rng.unit() < grit * 0.01 {
                x += rng.noise() * 3.0;
            }
            let pat = bp.tick(x, scuff) * env(t, 0.008, length) * 2.2;
            soft.tick(weight + pat, muffle)
        })
        .collect()
}

fn swish(low: f32, high: f32, length: f32, q: f32, whistle: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length, sr);
    let (mut body, mut air, mut edge) = (Filter::new(Pass::Band, q, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Band, 12.0, sr));
    let (low, high) = (low * rng.range(0.9, 1.1), high * rng.range(0.9, 1.1));
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            // Up fast to the peak (a third in), then away.
            let e = if k < 0.35 { (k / 0.35).powi(2) } else { (-(k - 0.35) * 6.0).exp() };
            let hz = low + (high - low) * e;
            let x = rng.noise();
            body.tick(x, hz) * e * 2.2 + air.tick(x, 3500.0) * e * e * 0.08 + edge.tick(x, hz * 1.8) * e.powi(3) * whistle * 1.6
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn impact(body: f32, crack: f32, flesh: f32, slice: f32, ring: f32, length: f32, muffle: f32, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let n = frames(length + 0.05, sr);
    let (mut snap, mut wet, mut hiss, mut soft) = (Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Band, 1.0, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Low, 0.6, sr));
    let (body, flesh) = (body * rng.range(0.9, 1.1), flesh * rng.range(0.85, 1.15));
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let x = rng.noise();
            let c = snap.tick(x, 1200.0) * env(t, 0.001, 0.012) * crack * 2.0;
            ph += body * (1.0 + 0.6 * (-t / 0.025).exp()) / sr;
            let punch = (TAU * ph).sin() * env(t, 0.003, length * 0.6) * 0.4;
            let thwack = wet.tick(x, flesh * (1.0 - 0.4 * (t / length).min(1.0))) * env(t, 0.004, length) * 3.4;
            let cut = hiss.tick(x, 4000.0) * env(t, 0.004, 0.06) * slice;
            let glass = ring * 0.2 * ((TAU * 1800.0 * t).sin() + 0.5 * (TAU * 2520.0 * t).sin()) * env(t, 0.004, 0.18);
            soft.tick((1.4 * (c + punch + thwack + cut + glass)).tanh(), muffle)
        })
        .collect()
}

fn chop_rise() -> f32 {
    0.002
}

fn cut_tail() -> f32 {
    0.06
}

/// `Make::Cut`'s parts.
struct CutParts {
    low: f32,
    high: f32,
    length: f32,
    tail: f32,
    tear: f32,
    snap: f32,
    meat: f32,
    crunch: f32,
}

fn cut(p: &CutParts, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let CutParts { low, high, length, tail, tear, snap, meat, crunch } = *p;
    let n = frames(length + tail * 1.6 + 0.01, sr);
    let (low, high) = (low * rng.range(0.92, 1.08), high * rng.range(0.92, 1.08));
    let (mut band, mut edge, mut hp, mut hp2, mut top, mut body) = (Filter::new(Pass::Band, 2.0, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Low, 0.7, sr), Filter::new(Pass::Band, 1.2, sr));
    // Crunch: dense, short, lower gritty bursts through the contact.
    let mut grit = vec![0.0f32; n];
    if crunch > 0.0 {
        let mut at = length;
        while at < length + tail * 0.8 {
            let (hz, len, loud) = (rng.range(600.0, 2000.0), rng.range(0.0005, 0.002), rng.range(0.4, 1.0) * (-(at - length) / (tail * 0.4)).exp());
            let mut f = Filter::new(Pass::Band, 2.0, sr);
            let start = frames(at, sr);
            for k in 0..frames(len * 3.0, sr) {
                if let Some(g) = grit.get_mut(start + k) {
                    *g += f.tick(rng.noise(), hz) * env(k as f32 / sr, 0.0001, len) * loud * crunch * 4.0;
                }
            }
            // (Sparse enough to hear each: a crackle, not a hiss.)
            at += -(1.0 - rng.unit()).max(1e-6).ln() / 380.0;
        }
    }
    // (The floor: 500 Hz, lower with meat, never near a thump's.)
    let floor = 500.0 - 200.0 * meat.min(1.0);
    // Tiny wet bursts after the contact.
    let mut grains = vec![0.0f32; n];
    if tear > 0.0 {
        let mut at = length + 0.002;
        while at < length + tail {
            let (hz, len, loud) = (rng.range(1200.0, 3500.0), rng.range(0.001, 0.003), rng.range(0.3, 1.0) * (-(at - length) / (tail * 0.5)).exp());
            let mut f = Filter::new(Pass::Band, 3.0, sr);
            let start = frames(at, sr);
            for k in 0..frames(len * 3.0, sr) {
                if let Some(g) = grains.get_mut(start + k) {
                    *g += f.tick(rng.noise(), hz) * env(k as f32 / sr, 0.0002, len) * loud * 0.9;
                }
            }
            at += -(1.0 - rng.unit()).max(1e-6).ln() / tear;
        }
    }
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let x = rng.noise();
            // In: the swing arriving, rising in pitch and loudness.
            let (hz, a) = if t < length {
                let k = t / length;
                (low + (high - low) * k * k, k.powf(2.5))
            } else {
                // Out: cut off sharply, the band falling.
                let k = (t - length) / tail.max(0.005);
                (high * (1.0 - 0.45 * k.min(1.0)), (-k * 4.0).exp() * 0.8)
            };
            let body_in = band.tick(x, hz) * a * 2.4;
            // The snap at the contact: a bright tick.
            let s = edge.tick(x, 3000.0) * env((t - length).max(0.0), 0.0003, 0.012) * if t >= length { snap * 2.2 } else { 0.0 };
            // The meat: a chunk of body at the contact.
            let m = body.tick(x, 850.0) * env((t - length).max(0.0), 0.002, tail * 0.9) * if t >= length { meat * 6.0 } else { 0.0 };
            // (A little drive with the crunch.)
            let y = body_in + s + m + grains[i] + grit[i];
            let y = if crunch > 0.0 { (y * (1.0 + 2.0 * crunch)).tanh() / (1.0 + crunch) } else { y };
            // (Nothing low: two high-passes at the floor; no hiss: nothing
            // much over 6 kHz.)
            top.tick(hp2.tick(hp.tick(y, floor), floor), 6000.0)
        })
        .collect()
}

/// `Make::Slash`'s parts.
struct SlashParts {
    chop: f32,
    weight: f32,
    edge: f32,
    wet: f32,
    tear: f32,
    slice: f32,
    soft: f32,
    length: f32,
    muffle: f32,
}

fn slash(p: &SlashParts, sr: f32, rng: &mut Rng) -> Vec<f32> {
    let SlashParts { chop, weight, edge, wet, tear, slice, soft, length, muffle } = *p;
    let n = frames(length * 1.4 + 0.03, sr);
    // Flesh parting: tiny wet bursts (1.5-5 ms, 0.7-2.6 kHz) scattered through
    // the chop, thinning out, each its own pitch and loudness.
    let mut grains = vec![0.0f32; n];
    if tear > 0.0 {
        let mut at = 0.004;
        while at < length {
            let (hz, len, loud) = (rng.range(700.0, 2600.0), rng.range(0.0015, 0.005), rng.range(0.3, 1.0) * (-at / (length * 0.5)).exp());
            let mut f = Filter::new(Pass::Band, 3.0, sr);
            let start = frames(at, sr);
            for k in 0..frames(len * 3.0, sr) {
                if let Some(g) = grains.get_mut(start + k) {
                    let t = k as f32 / sr;
                    *g += f.tick(rng.noise(), hz) * env(t, 0.0002, len) * loud * 1.3;
                }
            }
            at += -(1.0 - rng.unit()).max(1e-6).ln() / tear;
        }
    }
    let (mut bite, mut meat, mut under, mut lowpass, mut blade) = (Filter::new(Pass::Band, 1.6, sr), Filter::new(Pass::Band, 0.9, sr), Filter::new(Pass::Low, 0.7, sr), Filter::new(Pass::Low, 0.6, sr), Filter::new(Pass::Band, 2.2, sr));
    let (chop, weight) = (chop * rng.range(0.88, 1.12), weight * rng.range(0.9, 1.1));
    // (Each take its own flutter and a slightly different gap after the bite.)
    let (flutter, gap) = (rng.range(28.0, 46.0), rng.range(0.003, 0.007));
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let x = rng.noise();
            // The edge's bite: a bright tick, sweeping down.
            let bite_hz = 2400.0 + 3200.0 * (-t / 0.012).exp();
            let b = bite.tick(x, bite_hz) * env(t, 0.0004, 0.028) * edge * 2.6;
            // The chop, just after: falling, fluttering (wet).
            let tc = (t - gap).max(0.0);
            let on = if t < gap { 0.0 } else { 1.0 };
            let hz = chop * (1.35 - 0.7 * (tc / length).min(1.0));
            let squelch = 1.0 + wet * 0.7 * (TAU * flutter * tc).sin();
            let c = meat.tick(x, hz) * env(tc, soft, length * 0.75) * squelch * on * 4.2;
            // The cut: noise sweeping down from high as the blade goes
            // through.
            let s = blade.tick(x, 1800.0 + 3700.0 * (-t / 0.03).exp()) * env(t, 0.0015, 0.06) * slice * 3.0;
            // The weight behind it: a thump dropping in pitch.
            ph += weight * (1.0 + 2.0 * (-tc / 0.012).exp()) / sr;
            // (Its overtones carry it on small speakers, which lose what's
            // under ~150 Hz.)
            let thump = if weight > 0.0 { 1.0 } else { 0.0 };
            let w = ((TAU * ph).sin() + 0.6 * (2.0 * TAU * ph).sin() + 0.3 * (3.0 * TAU * ph).sin()) * env(tc, 0.002, length * 0.6) * on * 0.3 * thump;
            // A low rumble under it all (none without the thump).
            let r = under.tick(x, 320.0) * env(tc, 0.004, length * 1.1) * on * 1.2 * thump;
            lowpass.tick((1.6 * (b + s + c + w + r + grains[i])).tanh(), muffle)
        })
        .collect()
}

// ---- loops ----

/// Short events (crackles, drops, bubbles) at `rate` a second, each drawn
/// by `draw` into the buffer at its frame.
fn scatter(b: &mut Buf, rate: f32, sr: f32, rng: &mut Rng, mut draw: impl FnMut(&mut Buf, usize, &mut Rng)) {
    let n = b.len();
    let mut i = 0usize;
    if rate <= 0.0 {
        return;
    }
    loop {
        // (Poisson: exponential gaps.)
        let gap = -(1.0 - rng.unit()).max(1e-6).ln() / rate;
        i += frames(gap, sr).max(1);
        if i >= n {
            break;
        }
        draw(b, i, rng);
    }
}

fn fire(crackle: f32, roar: f32, hiss: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    let dt = 1.0 / sr;
    for (ch, seed) in [(0, 11u64), (1, 23)] {
        let mut r = Rng::new(rng.next() ^ seed);
        let (mut brown, mut lp, mut hp, mut floor) = (Brown::default(), Filter::new(Pass::Low, 0.7, sr), Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::High, 0.7, sr));
        let (mut breath, mut flare) = (Wander::new(&mut r, 0.5), Wander::new(&mut r, 1.5));
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            let (a, f) = (breath.tick(&mut r, dt), flare.tick(&mut r, dt));
            let low = floor.tick(lp.tick(brown.tick(&mut r), 400.0 + 600.0 * a), 140.0) * roar * 2.5 * (0.6 + 0.4 * a);
            let air = hp.tick(r.noise(), 4000.0) * hiss * (0.3 + 0.7 * f);
            *o = low + air;
        }
    }
    // Crackles: tiny bright bursts, a few loud; now and then a low pop.
    scatter(&mut b, crackle, sr, rng, |b, at, r| {
        let loud = 0.15 + 0.85 * r.unit().powi(4);
        let (hz, len, pan) = (r.range(1500.0, 6000.0), r.range(0.001, 0.006), r.noise() * 0.8);
        let mut f = Filter::new(Pass::Band, 2.0, sr);
        for k in 0..frames(len * 3.0, sr) {
            let t = k as f32 / sr;
            b.add(at + k, f.tick(r.noise(), hz) * env(t, 0.0002, len) * loud * 3.0, pan);
        }
    });
    scatter(&mut b, crackle * 0.04, sr, rng, |b, at, r| {
        let (hz, pan) = (r.range(120.0, 260.0), r.noise() * 0.6);
        for k in 0..frames(0.06, sr) {
            let t = k as f32 / sr;
            b.add(at + k, (TAU * hz * t).sin() * env(t, 0.001, 0.05) * 0.5, pan);
        }
    });
    b
}

fn cave(rumble: f32, wind: f32, hum: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    let dt = 1.0 / sr;
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let (mut brown, mut lp, mut bp) = (Brown::default(), Filter::new(Pass::Low, 0.7, sr), Filter::new(Pass::Band, 1.2, sr));
        let (mut sweep, mut swell) = (Wander::new(&mut r, 0.08), Wander::new(&mut r, 0.1));
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let low = lp.tick(brown.tick(&mut r), 90.0) * rumble;
            let (s, w) = (sweep.tick(&mut r, dt), swell.tick(&mut r, dt));
            let air = bp.tick(brown.tick(&mut r) * 0.5 + r.noise() * 0.15, 250.0 + 500.0 * s) * wind * (0.25 + 0.75 * w);
            let tone = hum * 0.05 * ((TAU * 55.0 * t).sin() + 0.6 * (TAU * 82.6 * t).sin()) * (0.6 + 0.4 * (TAU * t / 9.0).sin());
            *o = low + air + tone;
        }
    }
    b
}

fn rain(drops: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let (mut hp, mut lp) = (Filter::new(Pass::High, 0.7, sr), Filter::new(Pass::Low, 0.7, sr));
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            *o = lp.tick(hp.tick(r.noise(), 1200.0), 7000.0) * 0.35;
        }
    }
    scatter(&mut b, drops, sr, rng, |b, at, r| {
        let (hz, loud, pan) = (r.range(2000.0, 6000.0), 0.1 + 0.5 * r.unit().powi(3), r.noise());
        let mut f = Filter::new(Pass::Band, 3.0, sr);
        for k in 0..frames(0.01, sr) {
            let t = k as f32 / sr;
            b.add(at + k, f.tick(r.noise(), hz) * env(t, 0.0002, 0.004) * loud * 3.0, pan);
        }
    });
    b
}

fn wind(gust: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    let dt = 1.0 / sr;
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let (mut brown, mut bp, mut whistle) = (Brown::default(), Filter::new(Pass::Band, 0.9, sr), Filter::new(Pass::Band, 14.0, sr));
        let mut g = Wander::new(&mut r, 0.15);
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            let k = g.tick(&mut r, dt);
            let x = brown.tick(&mut r) * 0.6 + r.noise() * 0.2;
            let body = bp.tick(x, 200.0 + 900.0 * k) * (0.2 + 0.8 * k);
            let high = whistle.tick(r.noise(), 700.0 + 500.0 * k) * gust * k * k * 0.5;
            *o = body + high;
        }
    }
    b
}

fn water(bubbles: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    let dt = 1.0 / sr;
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let mut bp = Filter::new(Pass::Band, 3.0, sr);
        let mut w = Wander::new(&mut r, 6.0);
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            let k = w.tick(&mut r, dt);
            *o = bp.tick(r.noise(), 300.0 + 1100.0 * k) * (0.3 + 0.7 * k) * 0.8;
        }
    }
    scatter(&mut b, bubbles, sr, rng, |b, at, r| {
        let (f0, loud, pan) = (r.range(300.0, 1000.0), r.range(0.1, 0.4), r.noise() * 0.8);
        let mut ph = 0.0;
        for k in 0..frames(0.04, sr) {
            let t = k as f32 / sr;
            ph += f0 * (1.0 + 0.6 * (t / 0.02).min(1.0)) / sr;
            b.add(at + k, (TAU * ph).sin() * env(t, 0.001, 0.03) * loud, pan);
        }
    });
    b
}

fn lava(glops: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let (mut brown, mut lp, mut hp) = (Brown::default(), Filter::new(Pass::Low, 0.7, sr), Filter::new(Pass::High, 0.7, sr));
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            *o = lp.tick(brown.tick(&mut r), 120.0) * 0.8 + hp.tick(r.noise(), 5000.0) * 0.03;
        }
    }
    scatter(&mut b, glops, sr, rng, |b, at, r| {
        let (f0, pan) = (r.range(50.0, 90.0), r.noise() * 0.7);
        let mut ph = 0.0;
        for k in 0..frames(0.2, sr) {
            let t = k as f32 / sr;
            ph += f0 * (1.0 + 1.2 * (t / 0.08).min(1.0)) / sr;
            b.add(at + k, ((TAU * ph).sin() + r.noise() * 0.2) * env(t, 0.005, 0.15) * 0.6, pan);
        }
    });
    b
}

fn thrust(roar: f32, hiss: f32, flutter: f32, secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    let dt = 1.0 / sr;
    for ch in 0..2 {
        let mut r = Rng::new(rng.next());
        let (mut brown, mut lp, mut bp, mut floor) = (Brown::default(), Filter::new(Pass::Low, 0.8, sr), Filter::new(Pass::Band, 0.8, sr), Filter::new(Pass::High, 0.7, sr));
        let mut drift = Wander::new(&mut r, 3.0);
        let mut ph = r.unit();
        let out = if ch == 0 { &mut b.l } else { &mut b.r };
        for o in out.iter_mut() {
            let d = drift.tick(&mut r, dt);
            ph = (ph + flutter * (0.9 + 0.2 * d) * dt).fract();
            let flap = 0.7 + 0.3 * (TAU * ph).sin();
            // (Not all under 150 Hz: small speakers would lose it.)
            let low = floor.tick(lp.tick(brown.tick(&mut r), 650.0 + 300.0 * d), 160.0) * roar * 2.5;
            let air = bp.tick(r.noise(), 1600.0 + 800.0 * d) * hiss;
            *o = (low + air) * flap;
        }
    }
    b
}

#[allow(clippy::too_many_arguments)]
fn pads(root: f32, chords: &[Vec<f32>], chord: f32, bright: f32, plucks: f32, scale: &[f32], secs: f32, sr: f32, rng: &mut Rng) -> Buf {
    let n = frames(secs, sr);
    let mut b = Buf::zeros(n);
    if chords.is_empty() {
        return b;
    }
    let hz = |semis: f32| root * 2f32.powf(semis / 12.0);
    // Each chord over its slot and a fade either side (neighbours overlap).
    let (slot, fade) = (chord.max(1.0), 1.8f32);
    let slots = (secs / slot).ceil() as usize + 1;
    for s in 0..slots {
        let notes = &chords[s % chords.len()];
        let (start, end) = (s as f32 * slot - fade, (s + 1) as f32 * slot + fade);
        let (i0, i1) = (frames(start.max(0.0), sr), frames(end, sr).min(n));
        for ch in 0..2 {
            let mut f = Filter::new(Pass::Low, 0.9, sr);
            let detune = if ch == 0 { 0.9965 } else { 1.0035 };
            let phases: Vec<f32> = notes.iter().map(|_| rng.unit()).collect();
            let out = if ch == 0 { &mut b.l } else { &mut b.r };
            for (i, o) in out.iter_mut().enumerate().take(i1).skip(i0) {
                let t = i as f32 / sr;
                // A raised-cosine window over the slot and its fades.
                let w = if t < start + 2.0 * fade { 0.5 - 0.5 * (PI * (t - start) / (2.0 * fade)).cos() } else if t > end - 2.0 * fade { 0.5 - 0.5 * (PI * (end - t) / (2.0 * fade)).cos() } else { 1.0 };
                let mut x = 0.0;
                for (semis, ph) in notes.iter().zip(&phases) {
                    let f0 = hz(*semis);
                    let a = (f0 * t + ph).fract() * 2.0 - 1.0;
                    let c = (f0 * detune * t + ph * 0.7).fract() * 2.0 - 1.0;
                    x += (a + c) * 0.5;
                }
                // (And the lowest note an octave down, round.)
                x += 0.5 * (TAU * hz(notes[0]) * 0.5 * t).sin();
                let cutoff = bright * (0.75 + 0.25 * (TAU * t / 11.0 + ch as f32).sin());
                *o += f.tick(x * 0.3, cutoff) * w;
            }
        }
    }
    // Plucked notes from the scale, up high, now and then.
    if plucks > 0.0 && !scale.is_empty() {
        let beat = slot / 4.0;
        let beats = (secs / beat) as usize;
        for k in 0..beats {
            if rng.unit() >= plucks {
                continue;
            }
            let semis = scale[(rng.unit() * scale.len() as f32) as usize % scale.len()] + if rng.unit() < 0.5 { 12.0 } else { 24.0 };
            let (f0, pan, at) = (hz(semis), rng.noise() * 0.7, frames(k as f32 * beat, sr));
            for i in 0..frames(2.0, sr) {
                let t = i as f32 / sr;
                let x = (TAU * f0 * t).sin() + 0.2 * (TAU * f0 * 2.0 * t).sin();
                b.add(at + i, x * env(t, 0.004, 1.8) * 0.18, pan);
            }
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every recipe makes a sound: finite, not silent, levelled.
    #[test]
    fn every_recipe_renders_a_sound() {
        let sr = 48_000.0;
        let makes = [
            Make::Thud { pitch: 90.0, drop: 0.5, noise: 0.5, length: 0.2, bright: 2500.0 },
            Make::Clang { pitch: 900.0, ring: 0.3, length: 0.5, grit: 0.5, muffle: 4000.0 },
            Make::Whoosh { from: 300.0, to: 2000.0, length: 0.2, q: 1.0 },
            Make::Burst { cutoff: 1500.0, length: 0.08, q: 1.0, body: 120.0, grit: 0.5 },
            Make::Drip { pitch: 1400.0, rise: 0.5, length: 0.12 },
            Make::Boom { length: 1.2, sub: 1.0 },
            Make::Zap { pitch: 200.0, length: 0.3 },
            Make::Chime { pitch: 800.0, notes: vec![0.0, 7.0, 12.0], length: 0.8 },
            Make::Blip { pitch: 600.0, to: 1200.0, length: 0.1 },
            Make::Step { heel: 80.0, scuff: 900.0, q: 0.8, grit: 0.3, length: 0.1, muffle: 1500.0 },
            Make::Swish { low: 350.0, high: 2600.0, length: 0.22, q: 1.8, whistle: 0.4 },
            Make::Impact { body: 110.0, crack: 0.6, flesh: 1300.0, slice: 0.5, ring: 0.3, length: 0.14, muffle: 3000.0 },
            Make::Slash { chop: 900.0, weight: 80.0, edge: 0.6, wet: 0.5, tear: 200.0, slice: 0.5, soft: 0.01, length: 0.16, muffle: 5000.0 },
            Make::Cut { low: 900.0, high: 4200.0, length: 0.08, tail: 0.06, tear: 250.0, snap: 0.5, meat: 0.5, crunch: 0.5 },
            Make::Fire { crackle: 20.0, roar: 0.6, hiss: 0.1 },
            Make::Cave { rumble: 0.6, wind: 0.5, hum: 0.3 },
            Make::Rain { drops: 60.0 },
            Make::Wind { gust: 0.5 },
            Make::Water { bubbles: 12.0 },
            Make::Lava { glops: 1.5 },
            Make::Thrust { roar: 1.0, hiss: 0.4, flutter: 30.0 },
            Make::Pads { root: 110.0, chords: vec![vec![0.0, 7.0, 16.0], vec![5.0, 9.0, 12.0]], chord: 4.0, bright: 1200.0, plucks: 0.3, scale: vec![0.0, 2.0, 4.0, 7.0, 9.0] },
        ];
        for (i, m) in makes.iter().enumerate() {
            let loops = if i >= 14 { 6.0 } else { 0.0 };
            let mut b = render(m, sr, i as u64, loops);
            assert!(!b.is_empty(), "{m:?}: empty");
            assert!(b.l.iter().chain(&b.r).all(|x| x.is_finite()), "{m:?}: not finite");
            assert!(b.peak() > 1e-3, "{m:?}: silent");
            if loops > 0.0 {
                assert_eq!(b.len(), (loops * sr) as usize, "{m:?}: a loop keeps its length");
            }
            level(&mut b, 0.9, 0.0);
            assert!(b.peak() <= 0.98);
        }
    }
}
