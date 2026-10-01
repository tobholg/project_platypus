//! `platypus-bestiary` (`PLATYPUS_BESTIARY=<dir>`, the arena, no window):
//! the bestiary, written out for the model to read without opening the
//! game. For each creature (or those `PLATYPUS_BESTIARY_ONLY` names, comma
//! separated), its card opened as in the game: its numbers and details into
//! `<dir>/bestiary.md`, the panel as `<dir>/<id>_card.png`, and the live
//! stage caught once for each thing it shows (standing, walking, its
//! weapon, every phase of every move), put together as
//! `<dir>/<id>_strip.png` (half size, four to a row, numbered in the
//! markdown). The clock steps a tick a frame, so it's quicker than real
//! time and the same every run.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::time::TimeUpdateStrategy;

use super::panel::Bestiary;
use super::stage::Stage;

/// Each shown thing caught this long after it starts (s).
const SETTLE: f32 = 0.05;
/// A show cut off after this long (s).
const MOST: f32 = 30.0;
/// Frames to a row in a strip.
const ROW: u32 = 4;

pub struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut App) {
        let Ok(dir) = std::env::var("PLATYPUS_BESTIARY") else { return };
        let only: Vec<String> = std::env::var("PLATYPUS_BESTIARY_ONLY").map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default();
        let _ = std::fs::create_dir_all(PathBuf::from(&dir).join("frames"));
        app.insert_resource(Writer { dir: dir.into(), only, ..default() })
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / crate::world::TICK_HZ)))
            .add_systems(Update, write);
    }
}

#[derive(Resource, Default)]
struct Writer {
    dir: PathBuf,
    only: Vec<String>,
    /// The creatures to do, the one being done, since when, what it's
    /// showing and since when, what's been caught of it.
    todo: Option<Vec<String>>,
    doing: Option<String>,
    began: f32,
    caption: String,
    since: f32,
    caught: Vec<(PathBuf, String)>,
    carded: bool,
    /// The furthest into the show it's been (back at 0 after: round once).
    furthest: usize,
    /// Every strip to put together at the end, and the markdown.
    strips: Vec<(String, Vec<(PathBuf, String)>)>,
    md: String,
    /// When the last was done (the pictures still being written).
    done_at: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
fn write(mut commands: Commands, time: Res<Time>, mut w: ResMut<Writer>, mut b: ResMut<Bestiary>, stage: Option<Res<Stage>>, offscreen: Option<Res<crate::camera::Offscreen>>, mut exit: MessageWriter<AppExit>) {
    let now = time.elapsed_secs();
    if now < 1.5 {
        return;
    }
    let Some(stage) = stage else {
        error!("bestiary: no stage in this world (run it in the arena)");
        exit.write(AppExit::from_code(1));
        return;
    };
    // The list, once the catalogue's read.
    if w.todo.is_none() {
        let all: Vec<String> = b.shown().map(|e| e.id.clone()).filter(|id| w.only.is_empty() || w.only.contains(id)).collect();
        info!("bestiary: writing {} creatures to {}", all.len(), w.dir.display());
        w.md = format!("# Bestiary\n\n{} creatures, as the game knows them (`platypus-bestiary`).\n", all.len());
        w.todo = Some(all.into_iter().rev().collect());
        b.open = true;
    }
    // At the end: the strips put together (the pictures written by now).
    if let Some(at) = w.done_at {
        if now - at > 1.0 {
            let strips = std::mem::take(&mut w.strips);
            for (id, frames) in strips {
                if let Err(e) = strip(&w.dir.join(format!("{id}_strip.png")), &frames) {
                    warn!("bestiary: {id}'s strip: {e}");
                }
            }
            let md = w.dir.join("bestiary.md");
            if let Err(e) = std::fs::write(&md, &w.md) {
                error!("bestiary: {}: {e}", md.display());
            }
            info!("bestiary: written to {}", w.dir.display());
            exit.write(AppExit::Success);
            w.done_at = Some(f32::MAX);
        }
        return;
    }
    let caught_one = |commands: &mut Commands, w: &mut Writer, id: &str, caption: &str| {
        let path = w.dir.join("frames").join(format!("{id}_{:02}.png", w.caught.len()));
        commands.spawn(Screenshot::image(stage.image.clone())).observe(save_to_disk(path.clone()));
        w.caught.push((path, caption.to_string()));
    };
    // Next creature.
    let Some(id) = w.doing.clone() else {
        let next = w.todo.as_mut().and_then(|t| t.pop());
        let Some(id) = next else {
            w.done_at = Some(now);
            return;
        };
        b.search.clear();
        b.picked = Some(id.clone());
        w.doing = Some(id);
        w.began = now;
        w.caption.clear();
        w.caught.clear();
        w.carded = false;
        w.furthest = 0;
        return;
    };
    // Its card, once it's built and the stage has it.
    if !w.carded && now - w.began > 0.6 {
        w.carded = true;
        if let Some(o) = &offscreen {
            commands.spawn(Screenshot::image(o.0.clone())).observe(save_to_disk(w.dir.join(format!("{id}_card.png"))));
        }
        if let Some(e) = b.shown().find(|e| e.id == id) {
            let mut md = format!("\n## {} (`{}.ron`)\n\n![card]({id}_card.png)\n", e.name, e.id);
            for (head, lines) in e.details() {
                md += &format!("\n**{head}**\n\n");
                for l in lines {
                    md += &format!("- {}\n", l.trim());
                }
            }
            w.md += &md;
        }
    }
    // Each thing it shows, caught a moment in.
    if stage.showing.as_deref() == Some(id.as_str()) {
        // (Once round: the show's begun again.)
        if stage.beat() == 0 && w.furthest > 0 {
            finish(&mut w, &id);
            return;
        }
        w.furthest = w.furthest.max(stage.beat());
        if stage.caption != w.caption {
            w.caption.clone_from(&stage.caption);
            w.since = now;
        } else if now - w.since >= SETTLE && w.caught.last().is_none_or(|(_, c)| *c != w.caption) && !w.caption.is_empty() {
            let caption = w.caption.clone();
            caught_one(&mut commands, &mut w, &id, &caption);
        }
    }
    if now - w.began > MOST {
        finish(&mut w, &id);
    }
}

/// One creature done: its strip noted, the markdown told what's in it.
fn finish(w: &mut Writer, id: &str) {
    let frames = std::mem::take(&mut w.caught);
    let mut md = format!("\n**Live** ([strip]({id}_strip.png), left to right, top to bottom)\n\n");
    for (i, (_, c)) in frames.iter().enumerate() {
        md += &format!("{}. {c}\n", i + 1);
    }
    w.md += &md;
    w.strips.push((id.to_string(), frames));
    w.doing = None;
}

/// The frames at half size, `ROW` to a row, a dark line between.
fn strip(path: &std::path::Path, frames: &[(PathBuf, String)]) -> Result<(), String> {
    let pics: Vec<platypus_art::Pixels> = frames.iter().filter_map(|(p, _)| platypus_art::read_png(p).ok()).collect();
    let Some(first) = pics.first() else { return Err("no frames".into()) };
    let (fw, fh) = (first.w / 2, first.h / 2);
    let cols = ROW.min(pics.len() as u32);
    let rows = (pics.len() as u32).div_ceil(ROW);
    let mut out = platypus_art::Pixels::new(cols * (fw + 2), rows * (fh + 2));
    for y in 0..out.h as i32 {
        for x in 0..out.w as i32 {
            out.set(x, y, [20, 20, 26, 255]);
        }
    }
    for (i, p) in pics.iter().enumerate() {
        let (ox, oy) = ((i as u32 % ROW) * (fw + 2), (i as u32 / ROW) * (fh + 2));
        for y in 0..fh {
            for x in 0..fw {
                out.set((ox + x) as i32, (oy + y) as i32, p.get((x * 2) as i32, (y * 2) as i32));
            }
        }
    }
    platypus_art::write_png(&out, path)
}
