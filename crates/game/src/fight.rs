//! The arena's readouts (DESIGN §14.7, arena v2): the fight as it goes,
//! from every hurt anything takes (`creatures::Took`: blows, spells,
//! burning, falls): what's been done to them and to you, by kind (and how
//! it landed: hurt, shrugged off, drunk in), per second, and a timeline of
//! the last `TIMELINE_SECS` (to them above the line, to you below, each
//! kind its colour). A fight runs from its first hurt until `FIGHT_OVER`
//! seconds pass with none (the next starts afresh); the panel's "New
//! fight" starts one now.

use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::creatures::body::hurt::{Reaction, color_of, reaction};
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Harm, Took};

/// The timeline's grain (s), and how much of it shows.
const BIN: f32 = 0.1;
const TIMELINE_SECS: f32 = 20.0;
const BINS: usize = (TIMELINE_SECS / BIN) as usize;
/// The timeline's picture: a pixel a bin, half its height each way.
const HALF: u32 = 20;
/// Seconds with no hurt that end a fight.
const FIGHT_OVER: f32 = 5.0;
/// How often the readout is rewritten (s).
const SHOW_EVERY: f32 = 0.25;

/// What one side took, by kind (`Harm as usize`): meant, and got through.
#[derive(Clone, Copy, Debug, Default)]
pub struct Side {
    pub meant: [f32; 11],
    pub dealt: [f32; 11],
}

impl Side {
    pub fn total(&self) -> f32 {
        self.dealt.iter().sum()
    }

    fn add(&mut self, t: &Took) {
        self.meant[t.harm as usize] += t.meant;
        self.dealt[t.harm as usize] += t.dealt;
    }

    /// Its kinds, most first: "slash 120", "pierce 4 of 12" (shrugged
    /// off), "void drank 3".
    fn kinds(&self) -> String {
        let mut by: Vec<Harm> = Harm::ALL.into_iter().filter(|h| self.meant[*h as usize] > 0.0).collect();
        by.sort_by(|a, b| self.dealt[*b as usize].abs().total_cmp(&self.dealt[*a as usize].abs()));
        let line = |h: Harm| {
            let (m, d) = (self.meant[h as usize], self.dealt[h as usize]);
            match reaction(m, d) {
                Reaction::Hurt => format!("{} {d:.0}", h.name()),
                Reaction::Resisted => format!("{} {d:.0} of {m:.0}", h.name()),
                Reaction::Absorbed => format!("{} drank {:.0}", h.name(), -d),
            }
        };
        // (Three to a line: the panel's narrow, and a kind shouldn't wrap
        // away from its numbers.)
        let all: Vec<String> = by.into_iter().map(line).collect();
        all.chunks(3).map(|c| c.join("  ")).collect::<Vec<_>>().join("\n  ")
    }
}

/// The fight so far.
#[derive(Resource, Debug, Default)]
pub struct Fight {
    /// When it began and when it was last hurt (fixed time, s), and now.
    pub start: f32,
    pub last: f32,
    pub now: f32,
    pub on: bool,
    /// What they (everyone but the player) took, and what the player took.
    pub to_them: Side,
    pub to_you: Side,
    /// A bin of `BIN` s each, newest last: its start, what they and the
    /// player took in it, by kind.
    bins: VecDeque<(f32, [f32; 11], [f32; 11])>,
}

impl Fight {
    /// How long it's gone on (a single blow: a second).
    pub fn secs(&self) -> f32 {
        (self.last - self.start).max(1.0)
    }
}

/// Start a new fight now.
#[derive(Message, Clone, Copy, Debug)]
pub struct NewFight;

/// The readout's text, and the timeline's picture.
#[derive(Component)]
pub struct FightText;
#[derive(Resource)]
pub struct Timeline(pub Handle<Image>);

pub struct FightPlugin;

impl Plugin for FightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fight>()
            .add_message::<NewFight>()
            .add_systems(Startup, make_timeline)
            .add_systems(FixedUpdate, record.after(crate::creatures::tally).in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, show.run_if(on_timer(std::time::Duration::from_secs_f32(SHOW_EVERY))));
    }
}

use bevy::time::common_conditions::on_timer;

pub(crate) fn make_timeline(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d { width: BINS as u32, height: HALF * 2 + 1, depth_or_array_layers: 1 };
    let image = Image::new_fill(size, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    commands.insert_resource(Timeline(images.add(image)));
}

fn record(time: Res<Time>, mut fight: ResMut<Fight>, mut new: MessageReader<NewFight>, mut took: MessageReader<Took>, player: Query<(), With<LocalPlayer>>) {
    let now = time.elapsed_secs();
    fight.now = now;
    if new.read().count() > 0 || (fight.on && now - fight.last > FIGHT_OVER && !took.is_empty()) {
        *fight = Fight { now, ..default() };
    }
    for t in took.read() {
        if !fight.on {
            *fight = Fight { now, start: now, on: true, ..default() };
        }
        fight.last = now;
        let you = player.contains(t.target);
        if you { fight.to_you.add(t) } else { fight.to_them.add(t) }
        let at = now - (now - fight.start) % BIN;
        if fight.bins.back().is_none_or(|b| b.0 < at) {
            fight.bins.push_back((at, [0.0; 11], [0.0; 11]));
            if fight.bins.len() > BINS {
                fight.bins.pop_front();
            }
        }
        let bin = fight.bins.back_mut().expect("a bin");
        let side = if you { &mut bin.2 } else { &mut bin.1 };
        side[t.harm as usize] += t.dealt.abs();
    }
}

impl std::fmt::Display for Fight {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.on {
            return write!(f, "No fight yet: hurt something.");
        }
        let secs = self.secs();
        let (them, you) = (self.to_them.total(), self.to_you.total());
        let over = if self.now - self.last > FIGHT_OVER { "  (over)" } else { "" };
        writeln!(f, "{secs:.1} s{over}")?;
        writeln!(f, "To them  {them:.0}  {:.1}/s", them / secs)?;
        if them != 0.0 || self.to_them.meant.iter().any(|m| *m > 0.0) {
            writeln!(f, "  {}", self.to_them.kinds())?;
        }
        write!(f, "To you  {you:.0}  {:.1}/s", you / secs)?;
        if self.to_you.meant.iter().any(|m| *m > 0.0) {
            write!(f, "\n  {}", self.to_you.kinds())?;
        }
        Ok(())
    }
}

/// The readout and the timeline, a few times a second (while the panel's
/// open).
fn show(view: Res<crate::arena::ArenaView>, fight: Res<Fight>, timeline: Option<Res<Timeline>>, mut images: ResMut<Assets<Image>>, mut text: Query<&mut Text, With<FightText>>) {
    if !view.open {
        return;
    }
    for mut t in &mut text {
        t.0 = fight.to_string();
    }
    let Some(mut image) = timeline.and_then(|t| images.get_mut(&t.0)) else { return };
    let w = BINS as i32;
    let Some(data) = image.data.as_mut() else { return };
    data.fill(0);
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if (0..w).contains(&x) && (0..=2 * HALF as i32).contains(&y) {
            let i = ((y * w + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(&c);
        }
    };
    // The line, and a tick a second back from now.
    for x in 0..w {
        put(x, HALF as i32, if (w - 1 - x) % 10 == 0 { [140, 140, 150, 255] } else { [70, 70, 80, 200] });
    }
    // (The scale: the biggest bin shown; the root, so small hurts show.)
    let peak = fight.bins.iter().map(|b| b.1.iter().sum::<f32>().max(b.2.iter().sum())).fold(1.0, f32::max);
    let tall = |v: f32| ((v / peak).sqrt() * HALF as f32).round() as i32;
    for (at, them, you) in &fight.bins {
        let x = w - 1 - ((fight.now - at) / BIN).floor() as i32;
        for (side, up) in [(them, true), (you, false)] {
            let mut y = 0;
            let mut sum = 0.0;
            for h in Harm::ALL {
                let v = side[h as usize];
                if v <= 0.0 {
                    continue;
                }
                sum += v;
                let top = tall(sum).max(y + 1);
                let (r, g, b) = color_of(h);
                for dy in y..top {
                    put(x, if up { HALF as i32 - 1 - dy } else { HALF as i32 + 1 + dy }, [r, g, b, 255]);
                }
                y = top;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_side_tells_hurt_shrugged_and_drunk() {
        let mut s = Side::default();
        let t = |harm, meant, dealt| Took { target: Entity::PLACEHOLDER, harm, meant, dealt, killed: false };
        s.add(&t(Harm::Slash, 20.0, 20.0));
        s.add(&t(Harm::Pierce, 12.0, 4.0));
        s.add(&t(Harm::Void, 3.0, -3.0));
        assert_eq!(s.kinds(), "slash 20  pierce 4 of 12  void drank 3");
        assert_eq!(s.total(), 21.0);
    }
}
