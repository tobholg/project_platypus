//! Switching the player's lights (L): what it carries in its off hand and
//! on its head, off and on, without taking them off. Each goes as its kind
//! does:
//!
//! - **Electric** (a flashlight, a mining helmet's lamp): off, the beam
//!   drops at once to a warm glow that fades as a filament cools; on, it
//!   stutters into life (on, out, a weak flash, a brownout, full), a flare
//!   off the lens the first instant, a click each way.
//! - **A lantern**: its wick turned down to nothing and back up.
//! - **A torch**: snuffed (a puff of smoke) and lit again (a spark, the
//!   flame growing back).
//!
//! Off, a flame's out: it lights nothing, firedamp included (`hazards`),
//! and what hunts by light doesn't see you by it (`senses`).

use bevy::prelude::*;

use super::{Carry, LightToggles};
use crate::sound::PlaySound;

/// One light's switch: on or off, and how long since it was flipped (s).
#[derive(Clone, Copy, Debug)]
pub struct Lit {
    pub on: bool,
    pub since: f32,
    /// A roll per flip (each switching-on stutters its own way).
    salt: u32,
}

impl Default for Lit {
    fn default() -> Self {
        Lit { on: true, since: 100.0, salt: 0 }
    }
}

/// The player's lights' switches: the off hand's, the head's.
#[derive(Resource)]
pub struct Switch {
    pub hand: Lit,
    pub head: Lit,
}

impl Default for Switch {
    fn default() -> Self {
        // (Rolled apart: switched on together, each stutters its own way.)
        Switch { hand: Lit::default(), head: Lit { salt: 7_919, ..Lit::default() } }
    }
}

/// How long an electric light's stutter takes switching on, and its lens's
/// flare (s); how long its filament glows on after it's switched off.
const STUTTER: f32 = 0.42;
const FLARE: f32 = 0.12;
const AFTERGLOW: f32 = 0.22;
/// How long a flame takes to come back (a torch, a lantern) and to go down
/// (a lantern's; a torch is snuffed at once).
const TORCH_LIGHTS: f32 = 0.25;
const LANTERN_UP: f32 = 0.45;
const LANTERN_DOWN: f32 = 0.35;

/// A filament's glow, as it cools: its colour (warm, dim).
pub const FILAMENT: (u8, u8, u8) = (255, 140, 60);

impl Lit {
    /// An electric light: how bright (0..1), how much of that is a cooling
    /// filament's warm glow (0..1), and how much lens flare on top.
    pub fn electric(&self) -> (f32, f32, f32) {
        let t = self.since;
        if !self.on {
            return (0.18 * (-t / AFTERGLOW).exp(), 1.0, 0.0);
        }
        if t >= STUTTER {
            return (1.0, 0.0, 0.0);
        }
        // The stutter: a few steps, each a little longer or shorter than
        // the last time (by the flip's roll).
        let roll = |i: u32| (platypus_sim::rng::hash(&[self.salt as u64, i as u64]) % 1000) as f32 / 1000.0;
        let steps = [(0.05, 1.0), (0.06, 0.0), (0.04, 0.6), (0.09, 0.04), (0.04, 0.9), (0.05, 0.2)];
        let mut at = 0.0;
        for (i, (len, level)) in steps.iter().enumerate() {
            at += len * (0.7 + 0.6 * roll(i as u32));
            if t < at {
                let flare = if t < FLARE { 1.0 - t / FLARE } else { 0.0 };
                return (*level, 0.0, flare);
            }
        }
        // Then up to full.
        ((0.6 + 0.4 * ((t - at) / (STUTTER - at).max(0.01))).min(1.0), 0.0, 0.0)
    }

    /// A flame (a torch's, a lantern's): how much of it there is (0..1).
    pub fn flame(&self, lantern: bool) -> f32 {
        let t = self.since;
        match (self.on, lantern) {
            (true, false) => (t / TORCH_LIGHTS).min(1.0).powf(0.7),
            (true, true) => (t / LANTERN_UP).min(1.0),
            (false, true) => (1.0 - t / LANTERN_DOWN).max(0.0),
            (false, false) => 0.0,
        }
    }

    /// How many times it's been flipped (to notice a flip once).
    pub fn flips(&self) -> u32 {
        self.salt
    }

    fn flip(&mut self, on: bool) {
        self.on = on;
        self.since = 0.0;
        self.salt = self.salt.wrapping_add(1);
    }
}

impl Switch {
    /// How much light the off hand gives, as its kind goes (0: none: out, or
    /// nothing held).
    pub fn hand_level(&self, carry: Carry) -> f32 {
        match carry {
            Carry::Nothing => 0.0,
            Carry::SmallBeam | Carry::BigBeam => self.hand.electric().0,
            Carry::Torch => self.hand.flame(false),
            Carry::Lantern | Carry::SafetyLamp => self.hand.flame(true),
        }
    }
}

/// L: the lights carried off, or (any of them off) all on.
pub fn switch(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    taken: Res<crate::dev::KeyboardTaken>,
    dying: Res<crate::death::Dying>,
    toggles: Res<LightToggles>,
    mut switch: ResMut<Switch>,
    player: Query<&crate::creatures::Kinematics, With<crate::creatures::player::LocalPlayer>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let dt = time.delta_secs();
    switch.hand.since += dt;
    switch.head.since += dt;
    let hand = toggles.carry != Carry::Nothing;
    if !keys.just_pressed(KeyCode::KeyL) || taken.0 || dying.0.is_some() || !(hand || toggles.headlamp) {
        return;
    }
    let all_on = (!hand || switch.hand.on) && (!toggles.headlamp || switch.head.on);
    let on = !all_on;
    if hand {
        switch.hand.flip(on);
    }
    if toggles.headlamp {
        switch.head.flip(on);
    }
    let at = player.single().map_or(Vec2::ZERO, |k| k.body.pos);
    let (name, pitch, volume) = match (toggles.carry, on) {
        (Carry::Torch, true) => ("ignite", 1.0, 0.5),
        (Carry::Torch, false) => ("vent", 1.7, 0.3),
        (Carry::Lantern | Carry::SafetyLamp, _) => ("clink", 0.8, 0.3),
        (_, true) => ("clink", 1.9, 0.35),
        (_, false) => ("clink", 1.5, 0.3),
    };
    sounds.write(PlaySound::at(name, at).pitch(pitch).volume(volume));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_on_stutters_then_holds_and_off_glows_away() {
        let mut l = Lit::default();
        assert_eq!(l.electric().0, 1.0);
        l.flip(true);
        // In the stutter: out at least once, flared at the start.
        let levels: Vec<f32> = (0..40).map(|i| Lit { since: i as f32 * 0.01, ..l }.electric().0).collect();
        assert!(levels.iter().any(|&v| v < 0.1) && levels.iter().any(|&v| v > 0.8), "{levels:?}");
        assert!(Lit { since: 0.01, ..l }.electric().2 > 0.5);
        assert_eq!(Lit { since: 1.0, ..l }.electric().0, 1.0);
        l.flip(false);
        let glow = |t: f32| Lit { since: t, ..l }.electric().0;
        assert!(glow(0.0) > 0.1 && glow(0.3) < glow(0.0) && glow(1.5) < 0.01);
    }

    #[test]
    fn a_torch_is_out_at_once_and_a_lantern_turns_down() {
        let mut l = Lit::default();
        l.flip(false);
        assert_eq!(l.flame(false), 0.0);
        assert!(l.flame(true) > 0.9);
        assert_eq!(Lit { since: 1.0, ..l }.flame(true), 0.0);
        l.flip(true);
        assert!(l.flame(false) < 0.1 && Lit { since: 1.0, ..l }.flame(false) == 1.0);
    }
}
