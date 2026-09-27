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
    /// Holding down: drop through platforms.
    pub down: bool,
    /// -1..=1: up (+) or down (−), for swimming.
    pub move_y: f32,
    /// The world point the creature is aiming at (attacks, spells; the
    /// player's cursor); it faces it. Zero: not aiming (faces the way it
    /// moves).
    pub aim: Vec2,
    /// Its grappling hook's button (held): a press throws it, holding reels
    /// in.
    pub hook: bool,
}

/// How hard a climber presses into what it holds on to (cells/s).
const CLING_PRESS: f32 = 30.0;

/// On a rope, steering pushes at this share of gravity (a pendulum's own
/// scale, whatever the air control), while it's going slower than this
/// many times run speed that way.
const SWING_PUMP: f32 = 0.33;
const SWING_PUMP_MAX: f32 = 3.0;

/// Fastest a rope lets a body go (cells/s).
const TETHER_MAX: f32 = 900.0;

/// A rope from `at`, `len` long, holding a body at its other end: it goes
/// round `at` or toward it, never further. Speed outward is taken away (the
/// swing kept); where it would end up past the rope's end it's pulled back
/// onto it (a shortening rope, reeling in, pulls it). Returns whether the
/// rope is taut. Call after `steer`, before `move_and_collide`.
pub fn tether(body: &mut Body, at: Vec2, len: f32, dt: f32) -> bool {
    let r = body.pos - at;
    let d = r.length();
    let next = body.pos + body.vel * dt - at;
    if d < len && next.length() <= len {
        return false;
    }
    let n = if d > 1e-4 { r / d } else { Vec2::NEG_Y };
    let out = body.vel.dot(n);
    if out > 0.0 {
        body.vel -= n * out;
    }
    let next = body.pos + body.vel * dt - at;
    let nd = next.length();
    if nd > len && nd > 1e-4 {
        let onto = at + next / nd * len;
        body.vel = (onto - body.pos) / dt;
    }
    body.vel = body.vel.clamp_length_max(TETHER_MAX);
    true
}

/// Rocket boots under water: shares of their thrust and top speed.
const ROCKET_WATER: (f32, f32) = (0.5, 0.45);

/// Seconds after rocket boots stop firing before they start to refill.
const ROCKET_RECHARGE_DELAY: f32 = 0.25;

/// Below this share of its body under water (its head out), a jump leaves
/// the water as a jump does, at this share of a jump's speed.
const BREACH: f32 = 0.85;
const BREACH_SPEED: f32 = 1.0;

/// Per-creature movement tuning, loaded from creature RON files.
/// Units: cells and seconds.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MovementStats {
    pub run_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    /// Turning round on the ground (steering against how it's moving): a
    /// skid (0: as `ground_accel`).
    pub turn_accel: f32,
    pub air_accel: f32,
    pub gravity: f32,
    /// Gravity multiplier while falling (heavier fall = snappier jumps).
    pub fall_gravity: f32,
    pub max_fall: f32,
    /// Apex height of a full jump, in cells.
    pub jump_height: f32,
    /// Upward speed is multiplied by this when jump is released early.
    pub jump_cut: f32,
    /// Terraria's jump: for up to this many seconds while jump is held it
    /// rises at its take-off speed, no gravity, then arcs over (0: a plain
    /// arc). `jump_height` is still a full jump's apex.
    pub jump_hold: f32,
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
    /// Speed of one swim stroke (jump while submerged), toward where it
    /// steers (up if nowhere); held, another every `swim_stroke_every`
    /// seconds. Every press
    /// is a stroke.
    pub swim_stroke: f32,
    /// Seconds between strokes while jump is held in water.
    pub swim_stroke_every: f32,
    /// Flyers (birds, bats): 0 = can't fly. A flyer in the air (or steering
    /// up off the ground) flies: its velocity steers toward (move_x,
    /// move_y) × this, at `fly_accel`, sinking slowly when it isn't steering
    /// up or down (a glide).
    pub fly_speed: f32,
    pub fly_accel: f32,
    /// A climber (a spider): touching a wall or a ceiling it holds on and
    /// goes along it where it steers, at run speed; a jump lets go.
    pub cling: bool,
    /// A swimmer (a fish): under water it goes where it steers (move_x,
    /// move_y) × this, at `swim_accel`, weightless; 0: it swims as bodies do.
    pub swim_speed: f32,
    pub swim_accel: f32,
    /// Rocket boots (0: none): holding jump in the air, once the jump's rise
    /// has slowed (and any air jumps are spent), thrust up for up to this
    /// many seconds, refilled on landing (Terraria's).
    pub rocket_time: f32,
    /// Upward acceleration while it fires (cells/s², against gravity), up to
    /// `rocket_speed` rising.
    pub rocket_thrust: f32,
    pub rocket_speed: f32,
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
            turn_accel: 0.0,
            air_accel: 900.0,
            gravity: 1100.0,
            fall_gravity: 1.6,
            max_fall: 420.0,
            jump_height: 40.0,
            jump_cut: 0.45,
            jump_hold: 0.0,
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
            fly_speed: 0.0,
            fly_accel: 600.0,
            cling: false,
            swim_speed: 0.0,
            swim_accel: 400.0,
            swim_gravity: 0.1,
            swim_drag: 0.0005,
            swim_max_fall: 25.0,
            swim_stroke: 150.0,
            swim_stroke_every: 0.35,
            rocket_time: 0.0,
            rocket_thrust: 2600.0,
            rocket_speed: 230.0,
        }
    }
}

impl MovementStats {
    /// Take-off speed that reaches `jump_height` under `gravity` (held for
    /// `jump_hold` first: v·hold + v²/2g = height).
    pub fn jump_speed(&self) -> f32 {
        let (g, h) = (self.gravity.max(1.0), self.jump_hold.max(0.0));
        if h > 0.0 { g * (-h + (h * h + 2.0 * self.jump_height / g).sqrt()) } else { (2.0 * g * self.jump_height).sqrt() }
    }

    /// Straight up to a full jump's height with no hold (a bounce: the
    /// pogo).
    pub fn bounce_speed(&self) -> f32 {
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
    /// Seconds left of a jump's held rise (`jump_hold`).
    hold_left: f32,
    wall_lock: f32,
    stun: f32,
    prev_jump: bool,
    prev_dash: bool,
    /// Holding on to a wall or ceiling: which way its feet point (a climber;
    /// zero: on the wall behind).
    cling: Option<Vec2>,
    /// Seconds to the next swim stroke, while jump is held in water.
    stroke_left: f32,
    /// Rocket boots' fuel left (seconds).
    pub rocket_left: f32,
    /// Seconds since the rocket boots last fired (or were held on empty).
    rocket_rest: f32,
    /// On a rope (set before each `steer`): in the air it keeps its swing
    /// (air control pumps it, never brakes it).
    pub swinging: bool,
    /// A wall behind it (the background: a cave's, a built wall, a trunk;
    /// set before each `steer`): a climber can hold on to it anywhere and
    /// crawl over it every way, as up a wall.
    pub backed: bool,
    /// A plunge (set for the next `steer`, then spent): it may fall this
    /// fast, past `max_fall`.
    pub dive: f32,
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
            hold_left: 0.0,
            wall_lock: 0.0,
            stun: 0.0,
            prev_jump: false,
            prev_dash: false,
            stroke_left: 0.0,
            rocket_left: 0.0,
            rocket_rest: 0.0,
            swinging: false,
            backed: false,
            dive: 0.0,
            cling: None,
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
    /// Rocket boots fired this tick.
    pub rocketed: bool,
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
        self.hold_left = 0.0;
    }

    /// Air jumps and the air dash back, as if it had landed (a pogo off
    /// something struck below).
    pub fn refresh_air(&mut self, s: &MovementStats) {
        self.air_jumps_left = s.air_jumps;
        self.air_dash_used = false;
    }

    /// Which way its feet point while it holds on to a wall or ceiling
    /// (`(0, 1)`: the ceiling; `(±1, 0)`: a wall), if it does.
    pub fn clinging(&self) -> Option<Vec2> {
        self.cling
    }

    pub fn is_dashing(&self) -> bool {
        self.state == MoveState::Dash
    }

    /// Is jump pressed this tick (held now, not last tick)?
    pub fn jump_pressed(&self, intent: &Intent) -> bool {
        intent.jump && !self.prev_jump
    }

    /// Jump off something that isn't the ground (letting go of a rope): at
    /// least `share` of a jump's speed up, whatever it had kept; the press
    /// is spent (no air jump from it too), and letting go of the button
    /// doesn't cut the rise (it's momentum, not a jump).
    pub fn leap(&mut self, s: &MovementStats, intent: &Intent, body: &mut Body, share: f32) {
        body.vel.y = body.vel.y.max(s.jump_speed() * share);
        self.prev_jump = intent.jump;
        self.buffer = 0.0;
        self.rising_from_jump = false;
        self.hold_left = 0.0;
        self.state = MoveState::Air;
    }

    /// Apply one tick of intent to `body.vel`. Call before `move_and_collide`,
    /// then `after_move` with the contacts it returned.
    pub fn steer(&mut self, s: &MovementStats, intent: &Intent, body: &mut Body, dt: f32) -> MoveEvents {
        let mut ev = MoveEvents::default();
        body.drop = intent.down;
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
        // Swimming (jump held in water) you tread water: no sinking.
        let treading = intent.jump && wet > 0.3;
        let gravity_scale = if treading { 1.0 - wet } else { 1.0 - wet * (1.0 - s.swim_gravity) };

        // Knocked back: physics only.
        if self.stun > 0.0 {
            self.stun -= dt;
            body.vel.y = (body.vel.y - s.gravity * s.fall_gravity * gravity_scale * dt).max(-s.max_fall);
            if grounded {
                body.vel.x = approach(body.vel.x, 0.0, s.ground_decel * self.contacts.grip * dt);
            }
            if self.stun <= 0.0 {
                self.state = if grounded { MoveState::Ground } else { MoveState::Air };
            }
            return ev;
        }

        if intent.move_x != 0.0 {
            self.facing = intent.move_x.signum();
        }
        // Aiming at something (the player at the cursor): it faces that,
        // whichever way it moves.
        if intent.aim != Vec2::ZERO {
            let d = intent.aim.x - body.pos.x;
            if d.abs() > 0.5 {
                self.facing = d.signum();
            }
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

        // A climber on a wall or ceiling: it holds on (pressing into it, so
        // it stays touching) and goes along it; a jump lets go (a leap).
        self.cling = None;
        if s.cling && !jump_pressed && self.contacts.submerged < 0.5 {
            let c = self.contacts;
            let wall = if c.wall_left { -1.0 } else if c.wall_right { 1.0 } else { 0.0 };
            let (mx, my) = (intent.move_x.clamp(-1.0, 1.0), intent.move_y.clamp(-1.0, 1.0));
            let on_ceiling = c.ceiling && !(wall != 0.0 && my < 0.0);
            // (On the ground, walking into a wall, it only climbs if it wants up.)
            let on_wall = wall != 0.0 && (!grounded || my > 0.0);
            if on_ceiling {
                body.vel = Vec2::new(mx * s.run_speed, CLING_PRESS);
                self.cling = Some(Vec2::Y);
            } else if on_wall {
                body.vel = Vec2::new(wall * CLING_PRESS, my * s.run_speed);
                self.cling = Some(Vec2::new(wall, 0.0));
            } else if self.backed && (!grounded || my > 0.0) {
                // On the wall behind: every way it steers (its feet point
                // into the picture: no side).
                body.vel = Vec2::new(mx, my).clamp_length_max(1.0) * s.run_speed;
                self.cling = Some(Vec2::ZERO);
            }
            if self.cling.is_some() {
                self.state = MoveState::Ground;
                self.air_jumps_left = s.air_jumps;
                self.rising_from_jump = false;
                return ev;
            }
        }

        // A swimmer under water: steered, weightless.
        if s.swim_speed > 0.0 && self.contacts.submerged > 0.5 {
            let steer = Vec2::new(intent.move_x.clamp(-1.0, 1.0), intent.move_y.clamp(-1.0, 1.0));
            let dv = steer * s.swim_speed - body.vel;
            body.vel += dv.clamp_length_max(s.swim_accel * dt);
            self.state = MoveState::Air;
            self.rising_from_jump = false;
            return ev;
        }

        // Fly: in the air (or taking off), steered by move_x/move_y.
        if s.fly_speed > 0.0 && (!grounded || intent.move_y > 0.0) && self.contacts.submerged < 0.5 {
            let steer = Vec2::new(intent.move_x.clamp(-1.0, 1.0), intent.move_y.clamp(-1.0, 1.0));
            let mut want = steer * s.fly_speed;
            if steer.y == 0.0 {
                want.y = -s.fly_speed * 0.15;
            }
            let dv = want - body.vel;
            body.vel += dv.clamp_length_max(s.fly_accel * dt);
            self.state = MoveState::Air;
            self.rising_from_jump = false;
            return ev;
        }

        // Run.
        let target = intent.move_x.clamp(-1.0, 1.0) * s.run_speed;
        // (On ice there's little grip to start, stop or turn with.)
        let turning = target * body.vel.x < 0.0 && s.turn_accel > 0.0;
        let accel = if grounded {
            (if turning { s.turn_accel } else if target != 0.0 { s.ground_accel } else { s.ground_decel }) * self.contacts.grip
        } else if self.wall_lock > 0.0 {
            s.air_accel * 0.25
        } else {
            s.air_accel
        };
        if self.swinging && !grounded {
            // On a rope: steering pumps the swing (up to a few times run
            // speed); nothing brakes it.
            let push = intent.move_x.clamp(-1.0, 1.0);
            if push != 0.0 && body.vel.x * push < s.run_speed * SWING_PUMP_MAX {
                body.vel.x += push * s.gravity * SWING_PUMP * dt;
            }
        } else {
            body.vel.x = approach(body.vel.x, target, accel * dt);
        }

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

        // Swim: in water jump is a stroke toward where it steers (up if
        // nowhere): one on the press, then another every so often while
        // it's held. Between strokes the water's drag slows you: a glide.
        let swimming = self.contacts.submerged > 0.3;
        // (Rocket boots with fuel: held jump under water fires them, not
        // strokes.)
        let rocket_ready = s.rocket_time > 0.0 && self.rocket_left > 0.0;
        self.stroke_left -= dt;
        // At the surface (its head out), a jump is a real jump: out of the
        // water, onto the bank.
        if swimming && jump_pressed && self.contacts.submerged < BREACH {
            body.vel.y = s.jump_speed() * BREACH_SPEED;
            self.stroke_left = s.swim_stroke_every;
            self.buffer = 0.0;
            self.rising_from_jump = true;
            ev.jumped = true;
        } else if swimming && intent.jump && (jump_pressed || (self.stroke_left <= 0.0 && !rocket_ready)) {
            let steer = Vec2::new(intent.move_x, intent.move_y);
            let dir = if steer.length_squared() > 0.01 { steer.normalize() } else { Vec2::Y };
            body.vel = body.vel * 0.35 + dir * s.swim_stroke;
            self.stroke_left = s.swim_stroke_every;
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
                self.hold_left = s.jump_hold;
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

        // Rocket boots: jump held in the air once the jump's rise has slowed
        // (and any air jumps are spent: a press uses those first); under
        // water too, held after a stroke, pushing through the water at
        // about half the thrust and speed.
        self.rocket_left = self.rocket_left.min(s.rocket_time);
        let (thrust, top) = if swimming { (s.rocket_thrust * ROCKET_WATER.0, s.rocket_speed * ROCKET_WATER.1) } else { (s.rocket_thrust, s.rocket_speed) };
        if s.rocket_time > 0.0
            && self.rocket_left > 0.0
            && intent.jump
            && !jump_pressed
            && !grounded
            && (swimming || (self.state == MoveState::Air && self.air_jumps_left == 0))
            && body.vel.y < top
        {
            self.rocket_left -= dt;
            self.rocket_rest = 0.0;
            body.vel.y = (body.vel.y + thrust * dt).min(top);
            // (The rocket, not the jump, now: letting go doesn't cut it.)
            self.rising_from_jump = false;
            ev.rocketed = true;
        } else if s.rocket_time > 0.0 {
            // Not firing (anywhere: the ground, the air, water) they fill
            // back up over as long as they fire, a second's charge a second,
            // a moment after they stop; not while jump is held on empty in
            // the air (it'd sputter on for ever).
            let holding_empty = intent.jump && !grounded && self.rocket_left <= 0.0;
            self.rocket_rest = if holding_empty { 0.0 } else { self.rocket_rest + dt };
            if self.rocket_rest > ROCKET_RECHARGE_DELAY {
                self.rocket_left = (self.rocket_left + dt).min(s.rocket_time);
            }
        }

        // Gravity (none while a jump's rise is held).
        let holding = self.hold_left > 0.0 && intent.jump && body.vel.y > 0.0 && self.rising_from_jump;
        self.hold_left = if holding { self.hold_left - dt } else { 0.0 };
        let g = s.gravity * if body.vel.y < 0.0 { s.fall_gravity } else { 1.0 } * gravity_scale;
        if !holding {
            body.vel.y -= g * dt;
        }
        let max_fall = if self.state == MoveState::WallSlide { s.wall_slide_speed } else { s.max_fall.max(self.dive) };
        body.vel.y = body.vel.y.max(-max_fall);
        if wet > 0.0 {
            body.vel *= s.swim_drag.powf(dt * wet);
            // Sink slowly, not at falling speed (unless swimming down).
            if !treading {
                let max_sink = max_fall + (s.swim_max_fall - max_fall) * wet;
                body.vel.y = body.vel.y.max(-max_sink);
            }
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

    /// Run, let go: on ice it slides on long after; on stone it stops.
    #[test]
    fn ice_is_slippery() {
        let slide = |floor: &str| {
            let mut rows = vec!["#                                                                                                  #"; 80];
            let f = floor.repeat(100);
            rows.push(&f);
            let g = Ascii::new(&rows);
            let (s, mut l, mut b) = player();
            settle(&g, &s, &mut l, &mut b);
            for _ in 0..40 {
                tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, ..default_intent() });
            }
            let at = b.pos.x;
            for _ in 0..60 {
                tick(&g, &s, &mut l, &mut b, Intent::default());
            }
            b.pos.x - at
        };
        let (stone, ice) = (slide("#"), slide("="));
        assert!(stone < 6.0, "stone stops it: {stone}");
        assert!(ice > stone * 3.0 + 10.0, "ice slides it on: {ice} (stone {stone})");
    }

    /// Rocket boots: holding jump climbs far past a jump's height, for as
    /// long as the fuel lasts; not firing (in the air too), it fills again
    /// over as long as it fires (not at once on landing).
    #[test]
    fn rocket_boots_climb_while_jump_is_held() {
        let mut rows = vec!["#                                                                                                  #"; 400];
        rows.push("####################################################################################################");
        let g = Ascii::new(&rows);
        let (mut s, mut l, mut b) = player();
        s.air_jumps = 0;
        let apex = |s: &MovementStats, l: &mut Locomotion, b: &mut Body, frames: usize| {
            settle(&g, s, l, b);
            let floor = b.bottom();
            let mut top = floor;
            for _ in 0..frames {
                tick(&g, s, l, b, Intent { jump: true, ..default_intent() });
                top = top.max(b.bottom());
            }
            top - floor
        };
        let plain = apex(&s, &mut l, &mut b, 90);
        s.rocket_time = 1.0;
        let (mut l2, mut b2) = (Locomotion::default(), Body::new(Vec2::new(20.0, 10.0), Vec2::new(8.0, 16.0)));
        let rocket = apex(&s, &mut l2, &mut b2, 80);
        assert!(rocket > plain * 2.0, "rocket {rocket} vs a jump {plain}");
        assert!(l2.rocket_left <= 0.0, "the fuel ran out (80 ticks in, still up)");
        // Let go, still high up: after a moment it fills as it falls, over
        // as long as it fires (not at once).
        let delay = (ROCKET_RECHARGE_DELAY * 60.0).ceil() as usize;
        for _ in 0..delay {
            tick(&g, &s, &mut l2, &mut b2, Intent::default());
        }
        assert!(l2.rocket_left < 0.02, "not during the pause: {}", l2.rocket_left);
        for _ in 0..3 {
            tick(&g, &s, &mut l2, &mut b2, Intent::default());
        }
        assert!(!l2.grounded(), "still in the air");
        assert!((l2.rocket_left - 3.0 / 60.0).abs() < 0.02, "3 ticks after the pause, in the air: filling, {}", l2.rocket_left);
        for _ in 0..57 {
            tick(&g, &s, &mut l2, &mut b2, Intent::default());
        }
        assert!((l2.rocket_left - 1.0).abs() < 1e-3, "a second after it: full, {}", l2.rocket_left);
    }

    /// A body on a rope swings down and up the other side nearly as high
    /// as it started (the rope keeps its speed), and never past the rope's
    /// length.
    #[test]
    fn a_rope_swings_and_holds() {
        let mut rows = vec!["#                                                                                                                                                                                                      #"; 200];
        rows.push("########################################################################################################################################################################################################");
        let g = Ascii::new(&rows);
        let at = Vec2::new(100.0, 150.0);
        let mut b = Body::new(at + Vec2::new(-60.0, 0.0), Vec2::new(8.0, 16.0));
        let (mut top_right, mut far) = (f32::MIN, 0.0f32);
        let mut crossed = false;
        for _ in 0..240 {
            // (Falling freely: no steering to brake it.)
            b.vel.y -= 1100.0 * DT;
            tether(&mut b, at, 60.0, DT);
            move_and_collide(&g, &mut b, DT);
            far = far.max(b.pos.distance(at));
            if b.pos.x > at.x {
                crossed = true;
                top_right = top_right.max(b.pos.y);
            }
        }
        assert!(crossed, "it swung through");
        assert!(far < 61.5, "held at the rope's length: {far}");
        assert!(top_right > at.y - 12.0, "it swung up the far side to {top_right} (from {})", at.y);
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

    /// Terraria's jump: held, it rises at a steady speed for `jump_hold`,
    /// then arcs over, to the same height, longer in the air; let go early,
    /// it stops rising under power.
    #[test]
    fn a_held_jump_rises_steadily_then_arcs_to_the_same_height() {
        let g = room();
        let (base, _, _) = player();
        let s = MovementStats { jump_hold: 0.25, gravity: 514.0, fall_gravity: 1.0, jump_height: 38.0, jump_cut: 1.0, ..base };
        let (_, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        let floor = b.bottom();
        let (mut apex, mut air) = (floor, 0);
        let mut speeds = Vec::new();
        for t in 0..120 {
            tick(&g, &s, &mut l, &mut b, Intent { jump: true, ..default_intent() });
            apex = apex.max(b.bottom());
            if b.bottom() > floor + 0.5 {
                air = t;
            }
            speeds.push(b.vel.y);
        }
        assert!((apex - floor - 38.0).abs() < 3.0, "a held jump reached {} cells", apex - floor);
        assert!((speeds[5] - speeds[10]).abs() < 1.0, "steady while held: {} then {}", speeds[5], speeds[10]);
        assert!(air > 40, "in the air {air} ticks");
        // Let go at once: a hop.
        let (_, mut l, mut b) = player();
        settle(&g, &s, &mut l, &mut b);
        let mut hop = floor;
        for t in 0..90 {
            tick(&g, &s, &mut l, &mut b, Intent { jump: t < 2, ..default_intent() });
            hop = hop.max(b.bottom());
        }
        assert!(hop - floor < 20.0, "a tapped held-jump hopped {} cells", hop - floor);
    }

    /// Steering against its run on the ground: it turns at `turn_accel`.
    #[test]
    fn turning_round_skids_at_the_turn_rate() {
        let g = room();
        let (base, _, _) = player();
        let turn = |turn_accel: f32| {
            let s = MovementStats { ground_accel: 200.0, turn_accel, ..base.clone() };
            let (_, mut l, mut b) = player();
            settle(&g, &s, &mut l, &mut b);
            b.vel.x = s.run_speed;
            tick(&g, &s, &mut l, &mut b, Intent { move_x: -1.0, ..default_intent() });
            s.run_speed - b.vel.x
        };
        let (plain, skid) = (turn(0.0), turn(1000.0));
        assert!(skid > plain * 3.0, "a skid turns faster: {skid} vs {plain} a tick");
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

    /// Walking down a stair of small steps: it stays on the ground (walks
    /// down each), never a hop.
    #[test]
    fn walking_down_small_steps_stays_on_the_ground() {
        let mut rows = vec!["#                                                  #"; 20];
        // (A cell lower every 10 cells to the right.)
        rows.extend(["##########                                         #", "####################                               #", "##############################                     #", "########################################           #"]);
        rows.push("####################################################");
        let g = Ascii::new(&rows);
        let (s, mut l, mut b) = player();
        b.pos = Vec2::new(5.0, 20.0);
        settle(&g, &s, &mut l, &mut b);
        let mut airborne = 0;
        for _ in 0..40 {
            tick(&g, &s, &mut l, &mut b, Intent { move_x: -1.0, ..default_intent() });
        }
        b.pos.x = 5.0;
        settle(&g, &s, &mut l, &mut b);
        for _ in 0..40 {
            tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, ..default_intent() });
            if !l.grounded() {
                airborne += 1;
            }
        }
        assert!(b.pos.x > 40.0, "walked down the stair: {:?}", b.pos);
        assert_eq!(airborne, 0, "never off the ground");
    }

    #[test]
    fn coyote_time_allows_late_jumps_off_ledges() {
        // (A ledge taller than a step: off it, it falls; lower, it would
        // walk down onto the floor.)
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
            "###############                          #",
            "###############                          #",
            "###############                          #",
            "###############                          #",
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

    fn deep_pool() -> Ascii {
        let mut rows = vec!["#                                                                                                  #"; 10];
        rows.extend(vec!["#~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~#"; 90]);
        rows.push("####################################################################################################");
        Ascii::new(&rows)
    }

    /// Floating with its head out, a jump leaves the water like a jump; deep
    /// under, it's a stroke.
    #[test]
    fn a_jump_at_the_surface_clears_the_water() {
        let mut rows = vec!["#                              #"; 70];
        rows.extend(vec!["#~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~#"; 30]);
        rows.push("################################");
        let g = Ascii::new(&rows);
        let surface = 31.0;
        let rise = |bottom: f32| {
            let (s, mut l, mut b) = player();
            b.pos = Vec2::new(15.0, bottom + b.half.y);
            tick(&g, &s, &mut l, &mut b, Intent::default());
            let mut top = f32::MIN;
            for k in 0..60 {
                tick(&g, &s, &mut l, &mut b, Intent { jump: k < 20, ..Default::default() });
                top = top.max(b.bottom());
            }
            top - surface
        };
        let out = rise(surface - 12.0);
        assert!(out > 15.0, "head out, a jump clears the surface by {out} cells");
        let under = rise(surface - 25.0);
        assert!(under < out - 5.0, "deep under, a stroke gets less far: {under} vs {out}");
    }

    /// A climber walks up a wall, along the ceiling, and lets go when it
    /// jumps.
    #[test]
    fn a_climber_goes_up_walls_and_along_ceilings() {
        let mut rows = vec!["#                    #"; 30];
        rows.insert(0, "######################");
        rows.push("######################");
        let g = Ascii::new(&rows);
        let s = MovementStats { cling: true, run_speed: 40.0, ..Default::default() };
        let (_, mut l, _) = player();
        let mut b = Body::new(Vec2::new(10.0, 3.0), Vec2::new(4.0, 3.0));
        // Right, into the wall, and up it.
        for _ in 0..120 {
            tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, move_y: 1.0, ..Default::default() });
        }
        assert!(b.pos.y > 25.0, "climbed the wall to the top: {:?}", b.pos);
        assert!(l.clinging().is_some(), "holding on");
        // Left along the ceiling.
        for _ in 0..60 {
            tick(&g, &s, &mut l, &mut b, Intent { move_x: -1.0, move_y: 1.0, ..Default::default() });
        }
        assert!(b.pos.x < 15.0 && b.pos.y > 28.0, "along the ceiling: {:?}", b.pos);
        assert_eq!(l.clinging(), Some(Vec2::Y));
        // A jump lets go: it falls.
        tick(&g, &s, &mut l, &mut b, Intent { jump: true, ..Default::default() });
        for _ in 0..60 {
            tick(&g, &s, &mut l, &mut b, Intent::default());
        }
        assert!(b.pos.y < 5.0, "let go and fell: {:?}", b.pos);
    }

    /// A wall behind (the background): a climber crawls over it every way,
    /// out in the open, and doesn't fall; without it, it does.
    #[test]
    fn a_climber_crawls_over_the_wall_behind_it() {
        let mut rows = vec!["#                                        #"; 40];
        rows.push("##########################################");
        let g = Ascii::new(&rows);
        let s = MovementStats { cling: true, run_speed: 40.0, ..Default::default() };
        let (_, mut l, _) = player();
        let mut b = Body::new(Vec2::new(20.0, 3.0), Vec2::new(4.0, 3.0));
        for _ in 0..30 {
            l.backed = true;
            tick(&g, &s, &mut l, &mut b, Intent { move_x: 1.0, move_y: 1.0, ..Default::default() });
        }
        assert!(b.pos.y > 12.0 && b.pos.x > 30.0, "up and across the open wall: {:?}", b.pos);
        assert_eq!(l.clinging(), Some(Vec2::ZERO));
        // Still, it stays put.
        let at = b.pos;
        for _ in 0..30 {
            l.backed = true;
            tick(&g, &s, &mut l, &mut b, Intent::default());
        }
        assert!(b.pos.distance(at) < 0.5, "holds on: {:?} from {at:?}", b.pos);
        // Off the wall, it falls.
        for _ in 0..30 {
            l.backed = false;
            tick(&g, &s, &mut l, &mut b, Intent::default());
        }
        assert!(b.pos.y < at.y - 5.0, "fell off the end of it: {:?}", b.pos);
    }

    #[test]
    fn holding_jump_in_water_strokes_toward_where_you_steer() {
        let g = deep_pool();
        let (s, _, _) = player();
        let swim = |intent: Intent| {
            let (_, mut l, mut b) = player();
            b.pos = if intent.move_y < 0.0 { Vec2::new(50.0, 80.0) } else { Vec2::new(50.0, 45.0) };
            // Settle in the water first.
            for _ in 0..30 {
                tick(&g, &s, &mut l, &mut b, Intent::default());
            }
            let start = b.pos;
            // Held for 2 s, never let go.
            for _ in 0..120 {
                tick(&g, &s, &mut l, &mut b, intent);
            }
            b.pos - start
        };
        let up = swim(Intent { jump: true, ..Default::default() });
        assert!(up.y > 25.0, "held jump, it keeps stroking up ({:?})", up);
        let down = swim(Intent { jump: true, move_y: -1.0, ..Default::default() });
        assert!(down.y < -35.0, "down + jump dives ({:?})", down);
        let right = swim(Intent { jump: true, move_x: 1.0, ..Default::default() });
        assert!(right.x > 25.0 && right.y.abs() < 6.0, "right + jump swims right, level ({:?})", right);
    }
}
