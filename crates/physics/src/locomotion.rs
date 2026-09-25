//! How creatures move. A brain (keyboard or AI) writes an `Intent`; this turns
//! intent + `MovementStats` into velocity. The same code drives the player and
//! every enemy, so dash, coyote time, knockback and wall jumps work for all.
//!
//! Tuned for a Hollow Knight feel: snappy acceleration, heavier fall than
//! rise, variable jump height, forgiving timing windows.

use glam::Vec2;
use serde::Deserialize;

use crate::body::{Body, Contacts};

/// What a creature wants this tick. Buttons are *held* state; presses are
/// detected here, so an AI simply sets `jump = true` for a tick to jump.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intent {
    /// -1..=1
    pub move_x: f32,
    pub jump: bool,
    pub dash: bool,
    /// World-space direction the creature is aiming (attacks, guns).
    pub aim: Vec2,
}

/// Per-creature movement tuning, loaded from creature RON files.
/// Units: cells and seconds.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MovementStats {
    pub run_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    pub air_accel: f32,
    pub gravity: f32,
    /// Gravity multiplier while falling (heavier fall = snappier jumps).
    pub fall_gravity: f32,
    pub max_fall: f32,
    /// Apex height of a full jump, in cells.
    pub jump_height: f32,
    /// Upward speed is multiplied by this when jump is released early.
    pub jump_cut: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub air_jumps: u8,
    pub dash_speed: f32,
    pub dash_time: f32,
    pub dash_cooldown: f32,
    pub wall_jump: bool,
    pub wall_slide_speed: f32,
    /// Sideways speed given by a wall jump.
    pub wall_jump_push: f32,
    pub step_height: i32,
    /// Fraction of gravity felt when fully submerged.
    pub swim_gravity: f32,
    /// Velocity kept per second in liquid (drag). Water is thick: falling
    /// in at full speed, you stop within a body length or two.
    pub swim_drag: f32,
    /// Fastest sinking speed when fully submerged (cells/s).
    pub swim_max_fall: f32,
    /// Upward speed of one swim stroke (jump while submerged); every press
    /// is a stroke.
    pub swim_stroke: f32,
}

impl MovementStats {
    /// Sluggish (cold, mud, a slow spell): speeds, acceleration and jump
    /// scaled by `f` (0..1). Timing (coyote, buffers) is untouched.
    pub fn slowed(&self, f: f32) -> MovementStats {
        let f = f.clamp(0.05, 1.0);
        MovementStats {
            run_speed: self.run_speed * f,
            ground_accel: self.ground_accel * f,
            air_accel: self.air_accel * f,
            jump_height: self.jump_height * (0.5 + 0.5 * f),
            dash_speed: self.dash_speed * f,
            ..self.clone()
        }
    }
}

impl Default for MovementStats {
    fn default() -> Self {
        MovementStats {
            run_speed: 95.0,
            ground_accel: 1400.0,
            ground_decel: 1800.0,
            air_accel: 900.0,
            gravity: 1100.0,
            fall_gravity: 1.6,
            max_fall: 420.0,
            jump_height: 40.0,
            jump_cut: 0.45,
            coyote_time: 0.09,
            jump_buffer: 0.12,
            air_jumps: 0,
            dash_speed: 0.0,
            dash_time: 0.14,
            dash_cooldown: 0.45,
            wall_jump: false,
            wall_slide_speed: 60.0,
            wall_jump_push: 120.0,
            step_height: 3,
            swim_gravity: 0.25,
            swim_drag: 0.0005,
            swim_max_fall: 45.0,
            swim_stroke: 130.0,
        }
    }
}

impl MovementStats {
    /// Take-off speed that reaches `jump_height` under `gravity`.
    pub fn jump_speed(&self) -> f32 {
        (2.0 * self.gravity * self.jump_height).sqrt()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveState {
    #[default]
    Ground,
    Air,
    Dash,
    WallSlide,
    /// Knocked back; no control until the timer ends.
    Stunned,
}

/// Per-creature movement state. Timers count down in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Locomotion {
    pub state: MoveState,
    /// +1 right, -1 left.
    pub facing: f32,
    coyote: f32,
    buffer: f32,
    air_jumps_left: u8,
    dash_left: f32,
    dash_cooldown: f32,
    dash_dir: f32,
    air_dash_used: bool,
    rising_from_jump: bool,
    wall_lock: f32,
    stun: f32,
    prev_jump: bool,
    prev_dash: bool,
    /// Last tick's contacts, so brains and animation can read them.
    pub contacts: Contacts,
}

impl Default for Locomotion {
    fn default() -> Self {
        Locomotion {
            state: MoveState::Air,
            facing: 1.0,
            coyote: 0.0,
            buffer: 0.0,
            air_jumps_left: 0,
            dash_left: 0.0,
            dash_cooldown: 0.0,
            dash_dir: 1.0,
            air_dash_used: false,
            rising_from_jump: false,
            wall_lock: 0.0,
            stun: 0.0,
            prev_jump: false,
            prev_dash: false,
            contacts: Contacts::default(),
        }
    }
}

/// Events worth reacting to (sound, particles, animation).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveEvents {
    pub jumped: bool,
    pub air_jumped: bool,
    pub wall_jumped: bool,
    pub dashed: bool,
    /// Landing speed, if landed this tick.
    pub landed: Option<f32>,
}

impl Locomotion {
    pub fn grounded(&self) -> bool {
        self.contacts.ground
    }

    /// Knockback: set velocity and take control away for `secs`.
    pub fn knock(&mut self, body: &mut Body, velocity: Vec2, secs: f32) {
        body.vel = velocity;
        self.stun = self.stun.max(secs);
        self.state = MoveState::Stunned;
        self.dash_left = 0.0;
        self.rising_from_jump = false;
    }

    pub fn is_dashing(&self) -> bool {
        self.state == MoveState::Dash
    }

    /// Apply one tick of intent to `body.vel`. Call before `move_and_collide`,
    /// then `after_move` with the contacts it returned.
    pub fn steer(&mut self, s: &MovementStats, intent: &Intent, body: &mut Body, dt: f32) -> MoveEvents {
        let mut ev = MoveEvents::default();
        let jump_pressed = intent.jump && !self.prev_jump;
        let dash_pressed = intent.dash && !self.prev_dash;
        self.prev_jump = intent.jump;
        self.prev_dash = intent.dash;

        let grounded = self.contacts.ground;
        self.coyote = if grounded { s.coyote_time } else { (self.coyote - dt).max(0.0) };
        self.buffer = if jump_pressed { s.jump_buffer } else { (self.buffer - dt).max(0.0) };
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.0);
        self.wall_lock = (self.wall_lock - dt).max(0.0);
        if grounded {
            self.air_jumps_left = s.air_jumps;
            self.air_dash_used = false;
        }

        let wet = self.contacts.submerged;
        let gravity_scale = 1.0 - wet * (1.0 - s.swim_gravity);

        // Knocked back: physics only.
        if self.stun > 0.0 {
            self.stun -= dt;
            body.vel.y = (body.vel.y - s.gravity * s.fall_gravity * gravity_scale * dt).max(-s.max_fall);
            if grounded {
                body.vel.x = approach(body.vel.x, 0.0, s.ground_decel * dt);
            }
            if self.stun <= 0.0 {
                self.state = if grounded { MoveState::Ground } else { MoveState::Air };
            }
            return ev;
        }

        if intent.move_x != 0.0 {
            self.facing = intent.move_x.signum();
        }

        // Dash: fixed speed, no gravity, can't be steered.
        if dash_pressed && s.dash_speed > 0.0 && self.dash_cooldown <= 0.0 && (grounded || !self.air_dash_used) {
            self.state = MoveState::Dash;
            self.dash_left = s.dash_time;
            self.dash_dir = if intent.move_x != 0.0 { intent.move_x.signum() } else { self.facing };
            self.dash_cooldown = s.dash_cooldown;
            self.air_dash_used |= !grounded;
            self.rising_from_jump = false;
            ev.dashed = true;
        }
        if self.state == MoveState::Dash {
            self.dash_left -= dt;
            body.vel = Vec2::new(self.dash_dir * s.dash_speed, 0.0);
            if self.dash_left > 0.0 {
                return ev;
            }
            body.vel.x = self.dash_dir * s.run_speed;
            self.state = if grounded { MoveState::Ground } else { MoveState::Air };
        }

        // Run.
        let target = intent.move_x.clamp(-1.0, 1.0) * s.run_speed;
        let accel = if grounded {
            if target != 0.0 { s.ground_accel } else { s.ground_decel }
        } else if self.wall_lock > 0.0 {
            s.air_accel * 0.25
        } else {
            s.air_accel
        };
        body.vel.x = approach(body.vel.x, target, accel * dt);

        // Wall slide.
        let wall_dir = if self.contacts.wall_left { -1.0 } else if self.contacts.wall_right { 1.0 } else { 0.0 };
        let pushing_wall = wall_dir != 0.0 && intent.move_x.signum() == wall_dir;
        self.state = if grounded {
            MoveState::Ground
        } else if s.wall_jump && pushing_wall && body.vel.y <= 0.0 {
            MoveState::WallSlide
        } else {
            MoveState::Air
        };
        if self.state == MoveState::WallSlide {
            self.air_jumps_left = s.air_jumps;
            self.air_dash_used = false;
        }

        // Swim: in water every press of jump is a stroke upward.
        let swimming = self.contacts.submerged > 0.3;
        if swimming && jump_pressed {
            body.vel.y = body.vel.y.max(0.0) * 0.3 + s.swim_stroke;
            self.buffer = 0.0;
            ev.jumped = true;
        }

        // Jump (buffered, with coyote time).
        if self.buffer > 0.0 {
            let v = s.jump_speed();
            if grounded || self.coyote > 0.0 {
                body.vel.y = v;
                ev.jumped = true;
            } else if s.wall_jump && wall_dir != 0.0 {
                body.vel = Vec2::new(-wall_dir * s.wall_jump_push, v);
                self.facing = -wall_dir;
                self.wall_lock = 0.18;
                ev.wall_jumped = true;
            } else if self.air_jumps_left > 0 {
                self.air_jumps_left -= 1;
                body.vel.y = v * 0.9;
                ev.air_jumped = true;
            }
            if ev.jumped || ev.wall_jumped || ev.air_jumped {
                self.buffer = 0.0;
                self.coyote = 0.0;
                self.rising_from_jump = true;
                self.state = MoveState::Air;
            }
        }

        // Releasing jump early cuts the rise: short hops.
        if self.rising_from_jump && !intent.jump && body.vel.y > 0.0 {
            body.vel.y *= s.jump_cut;
            self.rising_from_jump = false;
        }
        if body.vel.y <= 0.0 {
            self.rising_from_jump = false;
        }

        // Gravity.
        let g = s.gravity * if body.vel.y < 0.0 { s.fall_gravity } else { 1.0 } * gravity_scale;
        body.vel.y -= g * dt;
        let max_fall = if self.state == MoveState::WallSlide { s.wall_slide_speed } else { s.max_fall };
        body.vel.y = body.vel.y.max(-max_fall);
        if wet > 0.0 {
            body.vel *= s.swim_drag.powf(dt * wet);
            // Sink slowly, not at falling speed.
            let max_sink = max_fall + (s.swim_max_fall - max_fall) * wet;
            body.vel.y = body.vel.y.max(-max_sink);
        }
        ev
    }

    /// Record contacts from `move_and_collide`; returns landing speed if landed.
    pub fn after_move(&mut self, contacts: Contacts) -> Option<f32> {
        let landed = (contacts.ground && !self.contacts.ground).then_some(contacts.impact);
        self.contacts = contacts;
        landed
    }
}

#[inline]
fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target { (v + step).min(target) } else { (v - step).max(target) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::move_and_collide;
    use crate::body::tests::Ascii;

    const DT: f32 = 1.0 / 60.0;

    fn room() -> Ascii {
        let mut rows = vec!["#                                                                                                  #"; 80];
        rows.push("####################################################################################################");
        Ascii::new(&rows)
    }

    fn tick(g: &Ascii, s: &MovementStats, l: &mut Locomotion, b: &mut Body, i: Intent) -> MoveEvents {
        let ev = l.steer(s, &i, b, DT);
        let c = move_and_collide(g, b, DT);
        l.after_move(c);
        ev
    }

    fn settle(g: &Ascii, s: &MovementStats, l: &mut Locomotion, b: &mut Body) {
        for _ in 0..60 {
            tick(g, s, l, b, Intent::default());
        }
        assert!(l.grounded());
    }

    fn player() -> (MovementStats, Locomotion, Body) {
        let s = MovementStats { dash_speed: 260.0, air_jumps: 1, wall_jump: true, ..Default::default() };
        let mut b = Body::new(Vec2::new(20.0, 10.0), Vec2::new(8.0, 16.0));
        b.step_height = s.step_height;
        (s, Locomotion::default(), b)
    }

    #[test]
    fn full_jump_reaches_the_configured_height() {
        let g = room();
        let (s, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        let floor = b.bottom();
        let mut apex = floor;
        for _ in 0..90 {
            tick(&g, &s, &mut l, &mut b, Intent { jump: true, ..default_intent() });
            apex = apex.max(b.bottom());
        }
        let h = apex - floor;
        assert!((h - s.jump_height).abs() < 3.0, "jumped {h} cells, wanted {}", s.jump_height);
    }

    #[test]
    fn tapping_jump_gives_a_short_hop() {
        let g = room();
        let (s, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        let floor = b.bottom();
        let mut apex = floor;
        for t in 0..90 {
            tick(&g, &s, &mut l, &mut b, Intent { jump: t < 3, ..default_intent() });
            apex = apex.max(b.bottom());
        }
        assert!(apex - floor < s.jump_height * 0.5, "short hop was {} cells", apex - floor);
    }

    #[test]
    fn jump_pressed_just_before_landing_is_buffered() {
        let g = room();
        let (s, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        b.pos.y += 6.0; // a few cells above ground, falling
        l.after_move(Default::default());
        let mut jumped = false;
        for t in 0..30 {
            // Press once while still airborne, then hold.
            let ev = tick(&g, &s, &mut l, &mut b, Intent { jump: t >= 1, ..default_intent() });
            jumped |= ev.jumped;
        }
        assert!(jumped, "buffered jump fired on landing");
    }

    #[test]
    fn coyote_time_allows_late_jumps_off_ledges() {
        let g = Ascii::new(&[
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "#                                        #",
            "###############                          #",
            "###############                          #",
            "#########################################",
        ]);
        let (s, mut l, mut b) = player();
        b.pos = Vec2::new(8.0, 12.0);
        settle(&g, &s, &mut l, &mut b);
        // Run off the ledge, press jump 3 ticks after leaving it.
        let mut left_at = None;
        let mut jumped = false;
        for t in 0..60 {
            let off = left_at.is_some_and(|l0| t >= l0 + 3);
            let ev = tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, jump: off, ..default_intent() });
            if left_at.is_none() && !l.grounded() {
                left_at = Some(t);
            }
            jumped |= ev.jumped;
        }
        assert!(jumped, "coyote jump");
    }

    #[test]
    fn dash_covers_a_fixed_distance_without_falling() {
        let g = room();
        let (s, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        b.pos.y += 20.0;
        l.after_move(Default::default());
        let (x0, y0) = (b.pos.x, b.pos.y);
        let ticks = (s.dash_time / DT).round() as usize;
        for t in 0..ticks {
            tick(&g, &s, &mut l, &mut b, Intent { dash: t == 0, ..default_intent() });
        }
        let dx = b.pos.x - x0;
        let expect = s.dash_speed * s.dash_time;
        assert!((dx - expect).abs() < s.dash_speed * DT * 1.5, "dashed {dx}, expected ~{expect}");
        assert!((b.pos.y - y0).abs() < 1.0, "no gravity during dash");
    }

    #[test]
    fn knockback_removes_control_temporarily() {
        let g = room();
        let (s, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        l.knock(&mut b, Vec2::new(-150.0, 100.0), 0.3);
        tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, ..default_intent() });
        assert!(b.vel.x < 0.0, "input ignored while stunned");
        for _ in 0..40 {
            tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, ..default_intent() });
        }
        assert!(b.vel.x > 0.0, "control returns");
    }

    fn default_intent() -> Intent {
        Intent::default()
    }

    #[test]
    fn falling_into_deep_water_slows_you_and_strokes_swim_you_up() {
        // A pool 60 deep (rows 1..=60) under 40 of air.
        let mut rows = vec!["#                                                                                                  #"; 40];
        rows.extend(vec!["#~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~#"; 60]);
        rows.push("####################################################################################################");
        let g = Ascii::new(&rows);
        let (s, mut l, mut b) = player();
        b.pos = Vec2::new(50.0, 95.0);
        b.vel = Vec2::new(0.0, -s.max_fall);
        let surface = 61.0;
        let mut depth_at_slow = None;
        for _ in 0..120 {
            tick(&g, &s, &mut l, &mut b, Intent::default());
            if b.pos.y < surface && b.vel.y > -s.swim_max_fall - 1.0 && depth_at_slow.is_none() {
                depth_at_slow = Some(surface - b.pos.y);
            }
        }
        let depth = depth_at_slow.expect("slowed in the water");
        assert!(depth < 40.0, "down to sinking speed within {depth} cells, well above the bottom");
        assert!(b.vel.y >= -s.swim_max_fall - 0.5, "sinks slowly: {}", b.vel.y);
        // Stroke up: tap jump.
        let start = b.pos.y;
        for k in 0..120 {
            tick(&g, &s, &mut l, &mut b, Intent { jump: k % 12 < 2, ..Default::default() });
        }
        assert!(b.pos.y > start + 10.0, "swam up from {start} to {}", b.pos.y);
    }
}
