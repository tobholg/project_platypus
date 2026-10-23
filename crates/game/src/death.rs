//! Dying, and the screen it puts up: what killed you (its picture, if it
//! was something; how; the blow and how far past dead it went; how often
//! that's had you), how this life went (how long, how deep, what you
//! killed and made, the gold left where you fell), the last moments (what
//! hurt you, by how much), a line for the stone, and a countdown to waking
//! (10 s; Space, pressed again, wakes you now). The world goes on behind it,
//! the camera where you fell; you're out of it till you wake (hidden, let
//! be, no hands). A scenario or a replay heals where it stands instead
//! (`respawn::heals_in_place`).
//!
//! What hurt it is kept by its `Health` (the player's: `Health::records`),
//! each hurt with a `Cause`: whatever deals harm says what it is
//! (`Health::harm_by`; plain `harm` names it by its kind).

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::creatures::Harm;

/// What hurt something: what to call it (a creature's name, "A cave-in"),
/// how ("bitten", "buried"), and the creature's kind if it was one (its
/// picture, its grudge).
#[derive(Clone, Debug, PartialEq)]
pub struct Cause {
    pub title: String,
    pub how: String,
    pub creature: Option<String>,
}

impl Cause {
    pub fn new(title: impl Into<String>, how: impl Into<String>) -> Cause {
        Cause { title: title.into(), how: how.into(), creature: None }
    }

    /// A creature's harm (by its kind and name), how by the kind of hurt.
    pub fn creature(kind: &str, name: &str, harm: Harm) -> Cause {
        let how = match harm {
            Harm::Slash => "slashed",
            Harm::Pierce => "pierced",
            Harm::Blunt => "battered",
            Harm::Fire => "burned",
            Harm::Frost => "frozen",
            Harm::Storm => "shocked",
            Harm::Acid => "dissolved",
            Harm::Poison => "poisoned",
            Harm::Radiant => "seared",
            Harm::Void => "drained",
            Harm::Fall => "thrown down",
        };
        Cause { title: name.to_string(), how: how.into(), creature: Some(kind.to_string()) }
    }

    /// A hurt nothing named: by its kind.
    pub fn of(harm: Harm) -> Cause {
        let (title, how) = match harm {
            Harm::Slash => ("A blade", "cut down"),
            Harm::Pierce => ("A sharp point", "run through"),
            Harm::Blunt => ("A heavy blow", "struck down"),
            Harm::Fire => ("Fire", "burned"),
            Harm::Frost => ("The cold", "frozen"),
            Harm::Storm => ("Lightning", "shocked"),
            Harm::Acid => ("Acid", "dissolved"),
            Harm::Poison => ("Poison", "poisoned"),
            Harm::Radiant => ("Light", "seared"),
            Harm::Void => ("The void", "unmade"),
            Harm::Fall => ("A fall", "fell"),
        };
        Cause::new(title, how)
    }
}

/// Who might have struck a blow (their kind and name, where they are,
/// their side).
pub type Who<'w, 's> = Query<'w, 's, (&'static crate::creatures::Creature, &'static crate::creatures::body::animation::Animator, &'static GlobalTransform, &'static crate::creatures::Team)>;

/// How far from where a blow landed its striker may be, when it isn't
/// known (cells): the nearest foe that near.
const BLAMED_REACH: f32 = 80.0;

/// What a blow was: who struck it (`by`, or the nearest foe to where it
/// landed), or else what it was.
pub fn blamed(who: &Who, by: Option<Entity>, at: Vec2, harm: Harm) -> Cause {
    use crate::creatures::Team;
    let named = |(c, a, _, team): (&crate::creatures::Creature, &crate::creatures::body::animation::Animator, &GlobalTransform, &Team)| {
        if *team == Team::Player { Cause::new("Yourself", "hurt") } else { Cause::creature(&c.kind, &a.def.name, harm) }
    };
    if let Some(e) = by
        && let Ok(w) = who.get(e)
    {
        return named(w);
    }
    who.iter()
        .filter(|(_, _, tf, team)| **team == Team::Enemy && tf.translation().truncate().distance(at) < BLAMED_REACH)
        .min_by(|a, b| a.2.translation().truncate().distance(at).total_cmp(&b.2.translation().truncate().distance(at)))
        .map_or_else(|| Cause::of(harm), named)
}

/// Blasts that know what they were (a TNT barrel, a bomb, a falling star),
/// for a little while after they're set off: a blast is felt the tick
/// after it goes off (`creatures::blasted`), and named by the nearest of
/// these to it.
#[derive(Resource, Default)]
pub struct NamedBlasts(Vec<(Vec2, Cause, u8)>);

/// How long a blast's name is kept (ticks), and how near it the blast it
/// names must be (cells).
const NAMED_FOR: u8 = 12;
const NAMED_NEAR: f32 = 8.0;

impl NamedBlasts {
    pub fn name(&mut self, at: Vec2, cause: Cause) {
        self.0.push((at, cause, NAMED_FOR));
    }

    /// What a blast at `at` was: its name, or the nearest foe to it, or an
    /// explosion.
    pub fn blamed(&self, who: &Who, at: Vec2, reach: f32) -> Cause {
        if let Some((_, c, _)) = self.0.iter().filter(|(p, ..)| p.distance(at) < NAMED_NEAR).min_by(|a, b| a.0.distance(at).total_cmp(&b.0.distance(at))) {
            return c.clone();
        }
        let near = blamed(who, None, at, Harm::Blunt);
        let foe = near.creature.is_some() && who.iter().any(|(c, _, tf, _)| Some(&c.kind) == near.creature.as_ref() && tf.translation().truncate().distance(at) < reach);
        if foe { near } else { Cause::new("An explosion", "blown up") }
    }
}

fn forget_blasts(mut named: ResMut<NamedBlasts>) {
    for b in &mut named.0 {
        b.2 = b.2.saturating_sub(1);
    }
    named.0.retain(|b| b.2 > 0);
}

/// How long a hurt is remembered (s): the last moments.
pub const REMEMBERED: f32 = 5.0;

/// The player's hurts age (and are forgotten past `REMEMBERED`).
fn age_hurts(time: Res<Time>, mut player: Query<&mut crate::creatures::Health, With<crate::creatures::player::LocalPlayer>>) {
    let dt = time.delta_secs();
    for mut h in &mut player {
        if h.recent.is_empty() {
            continue;
        }
        for r in &mut h.recent {
            r.age += dt;
        }
        h.recent.retain(|r| r.age <= REMEMBERED);
    }
}

/// One hurt remembered: what, how much got through, how long ago (s).
#[derive(Clone, Debug)]
pub struct Hurt {
    pub cause: Cause,
    pub dealt: f32,
    pub age: f32,
}

/// The last moments: what hurt it, summed by what (most first).
pub fn last_moments(recent: &[Hurt]) -> Vec<(String, f32)> {
    let mut by: BTreeMap<String, f32> = BTreeMap::new();
    for r in recent.iter().filter(|r| r.dealt > 0.0) {
        *by.entry(r.cause.title.clone()).or_default() += r.dealt;
    }
    let mut out: Vec<(String, f32)> = by.into_iter().collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

/// The player died (not a scenario's: `respawn::heals_in_place`), where,
/// and how far past dead the last blow took it.
#[derive(Message, Clone, Copy, Debug)]
pub struct PlayerDied {
    pub at: Vec2,
    pub over: f32,
}

/// The death screen, while it's up: the player's out of the world till it
/// wakes.
#[derive(Resource, Default)]
pub struct Dying(pub Option<Death>);

/// How long till you wake (s); how long after dying a Space press counts
/// (so mashing jump as you die doesn't skip it).
pub const COUNTDOWN: f32 = 10.0;
const SPACE_AFTER: f32 = 0.35;

/// A death, as the screen shows it.
#[derive(Clone, Debug)]
pub struct Death {
    pub at: Vec2,
    /// Since it died (s).
    pub since: f32,
    pub cause: Cause,
    /// The killing blow, and how far past dead it went.
    pub blow: f32,
    pub over: f32,
    /// How often what killed it has killed it (this time included).
    pub grudge: u32,
    pub moments: Vec<(String, f32)>,
    /// This life: how long (s; and whether that's the longest yet), how
    /// deep, what it killed (and most of), what it made, the gold it left.
    pub life: f32,
    pub record: bool,
    pub deepest: i32,
    pub kills: u32,
    pub most: Option<(String, u32)>,
    pub crafted: u32,
    pub gold: u32,
    pub epitaph: &'static str,
    /// The footer: deaths, the longest life, the day.
    pub deaths: u32,
    pub longest: f32,
    pub day: u32,
}

pub struct DeathPlugin;

/// Waking (and the screen): what follows a wake goes after this.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeathSet;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dying>()
            .init_resource::<NamedBlasts>()
            .add_message::<PlayerDied>()
            .add_systems(FixedUpdate, (age_hurts, forget_blasts, live, died).chain().after(crate::world::TickSet::Bodies))
            .add_systems(Update, (wake, screen::show).chain().in_set(DeathSet));
    }
}

/// The player's life ticks on (and it keeps what hurts it).
fn live(dying: Res<Dying>, pending: Option<Res<crate::save::PendingPlayer>>, mut player: Query<(&mut crate::progress::Progress, &mut crate::creatures::Health), With<crate::creatures::player::LocalPlayer>>) {
    let Ok((mut p, mut h)) = player.single_mut() else { return };
    if !h.records {
        h.records = true;
    }
    if dying.0.is_none() && pending.is_none() {
        p.bypass_change_detection().life += (1.0 / crate::world::TICK_HZ) as f32;
    }
}

/// Dying: what did it, how this life went, kept and put up; the gold half
/// left where it fell; the player out of the world (hidden, let be).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn died(
    mut commands: Commands,
    mut events: MessageReader<PlayerDied>,
    mut dying: ResMut<Dying>,
    creatures: Res<crate::creatures::def::Creatures>,
    daylight: Res<crate::light::Daylight>,
    mut queue: ResMut<crate::gold::GoldQueue>,
    mut player: Query<(Entity, &mut crate::progress::Progress, &mut crate::creatures::Health, Option<&mut crate::gold::Gold>), With<crate::creatures::player::LocalPlayer>>,
) {
    let Some(ev) = events.read().last().copied() else { return };
    let Ok((e, mut p, mut h, gold)) = player.single_mut() else { return };
    let last = h.recent.iter().rev().find(|r| r.dealt > 0.0).cloned();
    let (cause, blow) = last.map_or((Cause::new("Something", "killed"), 0.0), |r| (r.cause, r.dealt));
    let moments = last_moments(&h.recent);
    h.recent.clear();
    let p = &mut *p;
    p.deaths += 1;
    let grudge = {
        let n = p.grudges.entry(cause.title.clone()).or_default();
        *n += 1;
        *n
    };
    let life = p.life;
    let record = life > p.longest_life && p.deaths > 1;
    p.longest_life = p.longest_life.max(life);
    // What it killed this life (and most of), what it made.
    let mut kills = 0;
    let mut most: Option<(String, u32)> = None;
    for (kind, &n) in &p.kills {
        let this = n.saturating_sub(p.life_kills.get(kind).copied().unwrap_or(0));
        kills += this;
        if this > 0 && most.as_ref().is_none_or(|m| this > m.1) {
            let name = creatures.get(kind).map_or_else(|| kind.clone(), |d| d.name.clone());
            most = Some((name, this));
        }
    }
    let crafted = p.crafted.values().sum::<u32>().saturating_sub(p.life_crafted);
    // Half its gold, left where it fell (gold is a material: it bursts out
    // and lies there as specks, to be walked over and taken back).
    let gold = gold.map_or(0, |mut g| {
        let half = g.0 / 2;
        g.0 -= half;
        half
    });
    if gold > 0 {
        queue.burst.push((ev.at, gold));
    }
    commands.entity(e).insert((crate::creatures::brain::villager::Hiding, Visibility::Hidden));
    info!("death: {} ({}) for {blow:.0}, {:.0} past; this life {life:.0} s, {kills} kills, deepest {}; {gold} gold left at ({:.0}, {:.0})", cause.title, cause.how, ev.over, p.life_deepest, ev.at.x, ev.at.y);
    dying.0 = Some(Death {
        at: ev.at,
        since: 0.0,
        epitaph: epitaph(&cause, p.deaths),
        cause,
        blow,
        over: ev.over.max(0.0),
        grudge,
        moments,
        life,
        record,
        deepest: p.life_deepest,
        kills,
        most,
        crafted,
        gold,
        deaths: p.deaths,
        longest: p.longest_life,
        day: daylight.days.floor() as u32 + 1,
    });
}

/// The countdown, or a fresh Space press: waking at the checkpoint, the
/// life begun again.
#[allow(clippy::type_complexity)]
fn wake(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    sim: Res<crate::world::SimWorld>,
    checkpoint: Res<crate::respawn::Checkpoint>,
    mut dying: ResMut<Dying>,
    mut respawned: MessageWriter<crate::respawn::Respawned>,
    mut player: Query<(Entity, &mut crate::creatures::Kinematics, &mut crate::creatures::Health, &mut crate::progress::Progress), With<crate::creatures::player::LocalPlayer>>,
) {
    let Some(d) = dying.0.as_mut() else { return };
    d.since += time.delta_secs();
    let now = keys.just_pressed(KeyCode::Space) && d.since >= SPACE_AFTER;
    if d.since < COUNTDOWN && !now {
        return;
    }
    let died_at = d.at;
    dying.0 = None;
    let Ok((e, mut k, mut h, mut p)) = player.single_mut() else { return };
    let half = k.body.half;
    k.body.pos = crate::respawn::respawn_at(&sim, &checkpoint, half);
    k.body.vel = Vec2::ZERO;
    k.prev_pos = k.body.pos;
    h.hp = h.max;
    h.recent.clear();
    // (A new life: its clock, its depth, its tallies from here.)
    p.life = 0.0;
    p.life_deepest = i32::MIN;
    p.life_kills = p.kills.clone();
    p.life_crafted = p.crafted.values().sum();
    commands.entity(e).remove::<(crate::creatures::brain::villager::Hiding, crate::creatures::body::elements::Burning, crate::creatures::body::elements::Coated, crate::creatures::body::elements::Chilled)>().insert(Visibility::Inherited);
    respawned.write(crate::respawn::Respawned { died_at });
}

/// A line for the stone, by what did it (one of a few, by the count).
fn epitaph(cause: &Cause, deaths: u32) -> &'static str {
    let lines: &[&str] = match cause.title.as_str() {
        "Rat" | "Plague Rat" => &["The rats always win.", "Outnumbered by vermin. Outclassed, too."],
        "A cave-in" => &["Trusted a ceiling.", "Should have listened to the trickle."],
        "A gas explosion" => &["Brought a torch to a gas fight.", "The lamp went blue. Nobody was watching."],
        "A fall" => &["Gravity: still undefeated.", "Found the bottom."],
        "A wall" => &["Met a wall. The wall won."],
        "An explosion" | "A bomb" | "TNT Barrel" | "Dynamite" | "Mine Cart" => &["Stood a little too close.", "Went out with a bang."],
        "A slime" => &["Became part of the slime."],
        "Acid" => &["Dissolved, mostly."],
        "Flames" | "Searing heat" | "Fire" => &["Went out in a blaze. Literally.", "Too hot to handle."],
        "Lightning" | "Charged water" => &["Struck by inspiration. And lightning."],
        "Yourself" => &["Their own worst enemy.", "Friendly fire is still fire."],
        _ if cause.creature.is_some() => &["Fought bravely. Lost anyway.", "Should have packed a bigger sword.", "Underestimated the locals.", "The food chain is real."],
        _ => &["Here lies an explorer. The caves will wait.", "Rest now. There's always another way down."],
    };
    lines[deaths as usize % lines.len()]
}

/// The screen itself.
mod screen {
    use bevy::prelude::*;

    use super::{COUNTDOWN, Dying};

    #[derive(Component)]
    pub struct DeathScreen;

    #[derive(Component)]
    pub struct Countdown;

    const RED: Color = Color::srgb(0.82, 0.14, 0.1);
    const TEXT: Color = Color::srgb(0.92, 0.9, 0.86);
    const DIM: Color = Color::srgb(0.6, 0.57, 0.54);
    const FAINT: Color = Color::srgb(0.42, 0.4, 0.38);
    const BAR: Color = Color::srgb(0.7, 0.18, 0.12);
    const CARD: Color = Color::srgba(0.04, 0.02, 0.02, 0.82);

    fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, color: Color) {
        p.spawn((Text::new(s), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color)));
    }

    fn clock(secs: f32) -> String {
        let s = secs.max(0.0) as u32;
        if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
    }

    fn heading(p: &mut ChildSpawnerCommands, s: &str) {
        p.spawn(Node { margin: UiRect::top(px(14)), ..default() }).with_children(|h| text(h, s, 13.0, FAINT));
    }

    /// Put up (once a death's there), the countdown kept, taken down on waking.
    pub fn show(mut commands: Commands, dying: Res<Dying>, bestiary: Res<crate::bestiary::panel::Bestiary>, up: Query<Entity, With<DeathScreen>>, mut count: Query<&mut Text, With<Countdown>>, mut veil: Query<&mut BackgroundColor, With<DeathScreen>>) {
        let Some(d) = dying.0.as_ref() else {
            for e in &up {
                commands.entity(e).despawn();
            }
            return;
        };
        if !up.is_empty() {
            for mut t in &mut count {
                t.0 = format!("Resurrecting in {}", (COUNTDOWN - d.since).ceil().max(1.0) as u32);
            }
            // (Darkening as it comes up.)
            for mut bg in &mut veil {
                bg.0 = Color::srgba(0.03, 0.0, 0.0, 0.9 * (d.since / 0.6).min(1.0));
            }
            return;
        }
        commands
            .spawn((DeathScreen, GlobalZIndex(60), Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::NONE)))
            .with_children(|root| {
                root.spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(4), padding: UiRect::axes(px(48), px(28)), ..default() }, BackgroundColor(CARD))).with_children(|col| {
                    text(col, "YOU DIED", 64.0, RED);
                    // What did it.
                    col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(16), margin: UiRect::vertical(px(10)), ..default() }).with_children(|row| {
                        if let Some((img, size)) = d.cause.creature.as_deref().and_then(|k| bestiary.picture(k)) {
                            let scale = (84.0 / size.x.max(size.y) as f32).min(4.0);
                            row.spawn((Node { width: px(96), height: px(96), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)))).with_children(|f| {
                                f.spawn((ImageNode::new(img.clone()), Node { width: px(size.x as f32 * scale), height: px(size.y as f32 * scale), ..default() }));
                            });
                        }
                        row.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() }).with_children(|c| {
                            text(c, d.cause.title.clone(), 30.0, TEXT);
                            let over = if d.over >= 0.5 { format!(" ({:.0} too many)", d.over) } else { String::new() };
                            text(c, format!("{}, {:.0} damage{over}", d.cause.how, d.blow), 17.0, DIM);
                            let grudge = match d.grudge {
                                1 => "The first time it's had you".to_string(),
                                n => format!("It has killed you {n} times"),
                            };
                            text(c, grudge, 15.0, FAINT);
                        });
                    });
                    // This life.
                    heading(col, "THIS LIFE");
                    let record = if d.record { "  - new record!" } else { "" };
                    let deepest = if d.deepest > 0 { format!("{} below", d.deepest) } else { "the surface".into() };
                    let most = d.most.as_ref().map_or(String::new(), |(name, n)| format!(" (most: {name} x{n})"));
                    text(col, format!("Survived {}{record}", clock(d.life)), 18.0, TEXT);
                    text(col, format!("Deepest {deepest}   |   Kills {}{most}   |   Crafted {}", d.kills, d.crafted), 15.0, DIM);
                    if d.gold > 0 {
                        text(col, format!("{} gold left where you fell (marked on the map)", d.gold), 15.0, Color::srgb(0.95, 0.78, 0.3));
                    }
                    // The last moments.
                    if !d.moments.is_empty() {
                        heading(col, "LAST MOMENTS");
                        let most = d.moments[0].1.max(1.0);
                        for (what, much) in d.moments.iter().take(4) {
                            col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(10), ..default() }).with_children(|r| {
                                r.spawn(Node { width: px(150), justify_content: JustifyContent::End, ..default() }).with_children(|l| text(l, what.clone(), 15.0, DIM));
                                r.spawn((Node { width: px(220.0 * much / most), height: px(10), ..default() }, BackgroundColor(BAR)));
                                r.spawn(Node { width: px(40), ..default() }).with_children(|l| text(l, format!("{much:.0}"), 15.0, DIM));
                            });
                        }
                    }
                    col.spawn(Node { margin: UiRect::top(px(16)), ..default() }).with_children(|e| text(e, format!("\"{}\"", d.epitaph), 17.0, DIM));
                    col.spawn(Node { margin: UiRect::top(px(18)), ..default() }).with_children(|e| {
                        e.spawn((Countdown, Text::new(""), TextFont { font_size: FontSize::Px(24.0), ..default() }, TextColor(TEXT)));
                    });
                    text(col, "[Space] wake now", 15.0, DIM);
                    col.spawn(Node { margin: UiRect::top(px(16)), ..default() }).with_children(|e| text(e, format!("Deaths: {}   |   Longest life: {}   |   Day {}", d.deaths, clock(d.longest), d.day), 13.0, FAINT));
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creatures::Health;

    #[test]
    fn only_what_keeps_its_hurts_keeps_them_each_with_what_did_it() {
        let mut h = Health::new(50.0);
        h.harm(5.0, Harm::Fire);
        assert!(h.recent.is_empty(), "a creature doesn't keep them");
        h.records = true;
        h.harm_by(7.0, Harm::Blunt, || Cause::new("A cave-in", "buried"));
        h.harm(3.0, Harm::Fire);
        let kept: Vec<_> = h.recent.iter().map(|r| (r.cause.title.as_str(), r.dealt)).collect();
        assert_eq!(kept, vec![("A cave-in", 7.0), ("Fire", 3.0)]);
    }

    #[test]
    fn the_last_moments_add_up_by_what_most_first() {
        let hurt = |t: &str, d: f32| Hurt { cause: Cause::new(t, "x"), dealt: d, age: 0.0 };
        let m = last_moments(&[hurt("Rat", 3.0), hurt("Fire", 5.0), hurt("Rat", 4.0), hurt("Heal", -2.0)]);
        assert_eq!(m, vec![("Rat".to_string(), 7.0), ("Fire".to_string(), 5.0)]);
    }

    #[test]
    fn a_line_for_the_stone_fits_what_did_it() {
        assert_eq!(epitaph(&Cause::new("A cave-in", "buried"), 0), "Trusted a ceiling.");
        assert!(!epitaph(&Cause::creature("orc", "Orc", Harm::Slash), 3).is_empty());
        assert!(!epitaph(&Cause::of(Harm::Void), 1).is_empty());
    }
}
