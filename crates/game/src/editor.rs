//! The art editor, in the game beside the arena (its panel's "Art editor",
//! or E while the panel is open). It edits the sprite files in
//! `assets/art` as the text they are, through `platypus_art::edit`: a
//! stroke rewrites the rows it touched and nothing else, and is saved when
//! it ends, so creatures drawn from the file change in the world as you
//! draw, and the file stays one a person or the model can read and write
//! (the editor picks up their edits too).
//!
//! - Left: the files, and every frame, part, pose and derived frame in one.
//! - Middle: the canvas. A frame or part shows its own grid; a pose shows
//!   itself, and painting it paints the part of the layer picked on the
//!   right (where it lies, mirrored if it is). Onion skin: the frame before
//!   in its clip, faint. Points: a part's pivot (cyan) and points, a
//!   frame's anchors (yellow), the feet (red).
//! - Right: the palette (click a colour; R G B nudge it, Shift for single
//!   steps), a new colour, a pose's layers (arrows move the picked one),
//!   and a clip playing.
//!
//! Tools: pencil, eraser, fill, pick (also the right button), point (puts
//! the named point where you click). Ctrl or Cmd + Z / Y undo and redo;
//! Esc or E closes. While it's open it has the keyboard.
//!
//! `PLATYPUS_EDIT_DIR`: edit the sprites in another folder (the `editor`
//! scenario works on a scratch copy).

use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::input::InputSystems;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use platypus_art::edit::{self, Seg};
use platypus_art::{Art, ArtFile, Pixels};

use crate::data::{Watched, assets_dir};
use crate::dev::KeyboardTaken;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Editor>()
            .init_resource::<EditorKeys>()
            .add_systems(PreUpdate, capture.after(InputSystems))
            .add_systems(Update, (buttons, keys, paint, reload, build_ui, show).chain());
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Frame,
    Part,
    Pose,
    Derived,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Frame => "frame",
            Kind::Part => "part",
            Kind::Pose => "pose",
            Kind::Derived => "derived",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Tool {
    #[default]
    Pencil,
    Erase,
    Fill,
    Pick,
    Point,
}

/// What a button in the editor does.
#[derive(Clone, Debug, PartialEq)]
enum Act {
    File(usize),
    Item(usize),
    Color(char),
    NewColor,
    Channel(usize, i32),
    Tool(Tool),
    NextPoint,
    Onion,
    Undo,
    Redo,
    Layer(usize),
    Nudge(i32, i32),
    NextClip,
    Close,
}

#[derive(Resource, Default)]
pub struct Editor {
    pub open: bool,
    files: Vec<PathBuf>,
    file: usize,
    text: String,
    parsed: Option<ArtFile>,
    art: Option<Art>,
    error: Option<String>,
    status: String,
    items: Vec<(Kind, String)>,
    sel: usize,
    layer: usize,
    color: char,
    tool: Tool,
    point: usize,
    onion: bool,
    clip: usize,
    undo: Vec<String>,
    redo: Vec<String>,
    /// While painting: the text before the stroke (its undo), the last
    /// pixel painted.
    stroke: Option<(String, Option<(i32, i32)>)>,
    redraw: bool,
    /// What the built UI was built for.
    shape: String,
    watch: Option<Watched>,
    written: String,
    anim: f32,
    scroll: f32,
    hover: Option<(i32, i32)>,
}

/// The keyboard as the editor sees it (the game sees nothing while it's open).
#[derive(Resource, Default)]
pub(crate) struct EditorKeys(ButtonInput<KeyCode>);

/// A place in a grid: its path in the file, and how the canvas maps onto it.
#[derive(Clone, Debug)]
struct Target {
    /// `Some(name)`: a part's rows; `None`: the frame's own.
    part: Option<String>,
    frame: String,
    /// A pose layer's placement.
    layer: Option<Placed>,
}

/// Where a pose puts a part: its pivot at `at`, mirrored or not.
#[derive(Clone, Copy, Debug)]
struct Placed {
    at: (i32, i32),
    pivot: (i32, i32),
    flip: bool,
}

impl Target {
    fn path(&self) -> Vec<Seg<'_>> {
        match &self.part {
            Some(p) => vec![Seg::Field("parts"), Seg::Key(p), Seg::Field("rows")],
            None => vec![Seg::Field("frames"), Seg::Key(&self.frame)],
        }
    }

    /// Canvas pixel → grid pixel.
    fn map(&self, (x, y): (i32, i32)) -> (i32, i32) {
        match self.layer {
            None => (x, y),
            Some(Placed { at, pivot, flip }) => {
                let gx = if flip { pivot.0 - (x - at.0) } else { x - at.0 + pivot.0 };
                (gx, y - at.1 + pivot.1)
            }
        }
    }
}

impl Editor {
    fn path(&self) -> Option<&PathBuf> {
        self.files.get(self.file)
    }

    fn name(&self) -> String {
        self.path().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }

    fn find_files(&mut self) {
        // (PLATYPUS_EDIT_DIR: sprites somewhere else, a scratch copy.)
        let dir = std::env::var("PLATYPUS_EDIT_DIR").map(PathBuf::from).unwrap_or_else(|_| assets_dir().join("art"));
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ron")).collect())
            .unwrap_or_default();
        files.sort();
        // The player first: it's what's drawn most.
        if let Some(i) = files.iter().position(|p| p.file_stem().is_some_and(|s| s == "player")) {
            let p = files.remove(i);
            files.insert(0, p);
        }
        self.files = files;
    }

    fn open_file(&mut self, i: usize) {
        self.file = i;
        let Some(path) = self.path().cloned() else { return };
        match std::fs::read_to_string(&path) {
            Ok(t) => {
                self.text = t.clone();
                self.written = t;
                self.watch = Some(Watched::new(path));
                self.undo.clear();
                self.redo.clear();
                self.sel = 0;
                self.layer = 0;
                self.clip = 0;
                self.reparse();
                self.color = self.parsed.as_ref().and_then(|f| f.palette.keys().next().copied()).unwrap_or('.');
                self.status = format!("opened {}", self.name());
            }
            Err(e) => self.status = format!("{}: {e}", path.display()),
        }
    }

    fn reparse(&mut self) {
        let selected = self.items.get(self.sel).cloned();
        match platypus_art::parse(&self.text).and_then(|f| platypus_art::compile(&f).map(|a| (f, a))) {
            Ok((f, a)) => {
                let mut items = Vec::new();
                items.extend(f.poses.keys().map(|n| (Kind::Pose, n.clone())));
                items.extend(f.frames.keys().map(|n| (Kind::Frame, n.clone())));
                items.extend(f.derived.keys().map(|n| (Kind::Derived, n.clone())));
                items.extend(f.parts.keys().map(|n| (Kind::Part, n.clone())));
                self.items = items;
                if let Some(s) = selected
                    && let Some(i) = self.items.iter().position(|it| *it == s)
                {
                    self.sel = i;
                }
                self.sel = self.sel.min(self.items.len().saturating_sub(1));
                self.parsed = Some(f);
                self.art = Some(a);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        self.redraw = true;
    }

    /// New text: checked, kept, and (unless mid-stroke) saved.
    fn commit(&mut self, new: String, undoable: bool) {
        if new == self.text {
            return;
        }
        let old = std::mem::replace(&mut self.text, new);
        self.reparse();
        if self.error.is_some() {
            // (It wouldn't compile: not kept.)
            self.status = format!("not done: {}", self.error.take().unwrap_or_default());
            self.text = old;
            self.reparse();
            return;
        }
        if undoable {
            self.undo.push(old);
            self.redo.clear();
        }
        if self.stroke.is_none() {
            self.save();
        }
    }

    fn save(&mut self) {
        let Some(path) = self.path().cloned() else { return };
        match std::fs::write(&path, &self.text) {
            Ok(()) => {
                self.written = self.text.clone();
                self.status = format!("saved {}", self.name());
            }
            Err(e) => self.status = format!("not saved: {e}"),
        }
    }

    fn edit(&mut self, path: &[Seg], value: &str) {
        match edit::set(&self.text, path, value) {
            Ok(t) => self.commit(t, true),
            Err(e) => self.status = e,
        }
    }

    fn item(&self) -> Option<(Kind, String)> {
        self.items.get(self.sel).cloned()
    }

    /// The pose's layers, when a pose is picked.
    fn layers(&self) -> Vec<platypus_art::Layer> {
        match (self.item(), &self.parsed) {
            (Some((Kind::Pose, n)), Some(f)) => f.poses.get(&n).cloned().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// What painting edits.
    fn target(&self) -> Option<Target> {
        let (kind, name) = self.item()?;
        let f = self.parsed.as_ref()?;
        match kind {
            Kind::Frame => Some(Target { part: None, frame: name, layer: None }),
            Kind::Part => Some(Target { part: Some(name.clone()), frame: name, layer: None }),
            Kind::Pose => {
                let l = f.poses.get(&name)?.get(self.layer)?;
                let pivot = f.parts.get(&l.part)?.pivot;
                Some(Target { part: Some(l.part.clone()), frame: name, layer: Some(Placed { at: l.at, pivot, flip: l.flip }) })
            }
            Kind::Derived => None,
        }
    }

    fn grid(&self, t: &Target) -> Option<Vec<Vec<char>>> {
        let f = self.parsed.as_ref()?;
        let rows = match &t.part {
            Some(p) => &f.parts.get(p)?.rows,
            None => f.frames.get(&t.frame)?,
        };
        Some(rows.iter().map(|r| r.chars().collect()).collect())
    }

    fn set_grid(&mut self, t: &Target, g: &[Vec<char>], undoable: bool) {
        let rows: Vec<String> = g.iter().map(|r| r.iter().collect()).collect();
        match edit::set_rows(&self.text, &t.path(), &rows) {
            Ok(text) => self.commit(text, undoable),
            Err(e) => self.status = e,
        }
    }

    /// The names the point tool can put down here.
    fn point_names(&self) -> Vec<String> {
        let Some(f) = &self.parsed else { return Vec::new() };
        let mut v: Vec<String> = match self.item().map(|i| i.0) {
            Some(Kind::Frame) => f.anchors.keys().cloned().collect(),
            Some(Kind::Part | Kind::Pose) => std::iter::once("pivot".to_string()).chain(f.parts.values().flat_map(|p| p.points.keys().cloned())).collect(),
            _ => Vec::new(),
        };
        if !v.iter().any(|n| n == "hand") {
            v.push("hand".into());
        }
        v.dedup();
        let mut seen = std::collections::HashSet::new();
        v.retain(|n| seen.insert(n.clone()));
        v
    }

    fn point_name(&self) -> String {
        let names = self.point_names();
        names.get(self.point % names.len().max(1)).cloned().unwrap_or_else(|| "hand".into())
    }

    /// What the tool does at a canvas pixel (`fresh`: the button just went down).
    fn apply(&mut self, at: (i32, i32), fresh: bool, pick: bool) {
        let Some(t) = self.target() else { return };
        let Some(mut g) = self.grid(&t) else { return };
        let (x, y) = t.map(at);
        let inside = y >= 0 && x >= 0 && (y as usize) < g.len() && (x as usize) < g[y as usize].len();
        let tool = if pick { Tool::Pick } else { self.tool };
        match tool {
            Tool::Pencil | Tool::Erase if inside => {
                let c = if tool == Tool::Erase { platypus_art::CLEAR } else { self.color };
                if g[y as usize][x as usize] != c {
                    g[y as usize][x as usize] = c;
                    self.set_grid(&t, &g, false);
                }
            }
            Tool::Fill if inside && fresh => {
                let from = g[y as usize][x as usize];
                if from != self.color {
                    let mut todo = vec![(x, y)];
                    while let Some((x, y)) = todo.pop() {
                        let Some(c) = g.get_mut(y as usize).and_then(|r| r.get_mut(x as usize)) else { continue };
                        if x < 0 || y < 0 || *c != from {
                            continue;
                        }
                        *c = self.color;
                        todo.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
                    }
                    self.set_grid(&t, &g, true);
                }
            }
            Tool::Pick if inside && fresh => {
                let c = g[y as usize][x as usize];
                if c != platypus_art::CLEAR {
                    self.color = c;
                }
            }
            Tool::Point if fresh => {
                let name = self.point_name();
                let value = format!("({x}, {y})");
                match (&t.part, name.as_str()) {
                    (Some(p), "pivot") => self.edit(&[Seg::Field("parts"), Seg::Key(&p.clone()), Seg::Field("pivot")], &value),
                    (Some(p), n) => self.edit(&[Seg::Field("parts"), Seg::Key(&p.clone()), Seg::Field("points"), Seg::Key(n)], &value),
                    (None, n) => self.edit(&[Seg::Field("anchors"), Seg::Key(n), Seg::Key(&t.frame.clone())], &value),
                }
            }
            _ => {}
        }
    }

    fn act(&mut self, a: &Act, shift: bool) {
        match *a {
            Act::File(i) => self.open_file(i),
            Act::Item(i) => {
                self.sel = i;
                self.layer = 0;
                self.redraw = true;
            }
            Act::Color(c) => self.color = c,
            Act::NewColor => {
                let Some(f) = &self.parsed else { return };
                let pool = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let Some(c) = pool.chars().find(|c| !f.palette.contains_key(c)) else { return };
                let [r, g, b, _] = f.palette.get(&self.color).map_or([128, 128, 128, 255], |c| c.rgba());
                self.edit(&[Seg::Field("palette"), Seg::Char(c)], &format!("({r}, {g}, {b})"));
                self.color = c;
            }
            Act::Channel(ch, dir) => {
                let Some(f) = &self.parsed else { return };
                let Some(col) = f.palette.get(&self.color) else { return };
                let mut v = col.rgba();
                let step = if shift { 1 } else { 8 };
                v[ch] = (v[ch] as i32 + dir * step).clamp(0, 255) as u8;
                let value = match col {
                    platypus_art::Color::Rgb(..) => format!("({}, {}, {})", v[0], v[1], v[2]),
                    platypus_art::Color::Rgba(..) => format!("({}, {}, {}, {})", v[0], v[1], v[2], v[3]),
                };
                let c = self.color;
                self.edit(&[Seg::Field("palette"), Seg::Char(c)], &value);
            }
            Act::Tool(t) => self.tool = t,
            Act::NextPoint => self.point += 1,
            Act::Onion => {
                self.onion = !self.onion;
                self.redraw = true;
            }
            Act::Undo => {
                if let Some(t) = self.undo.pop() {
                    let now = std::mem::replace(&mut self.text, t);
                    self.redo.push(now);
                    self.reparse();
                    self.save();
                }
            }
            Act::Redo => {
                if let Some(t) = self.redo.pop() {
                    let now = std::mem::replace(&mut self.text, t);
                    self.undo.push(now);
                    self.reparse();
                    self.save();
                }
            }
            Act::Layer(i) => {
                self.layer = i;
                self.redraw = true;
            }
            Act::Nudge(dx, dy) => {
                let Some((Kind::Pose, pose)) = self.item() else { return };
                let Some(l) = self.layers().get(self.layer).cloned() else { return };
                let at = format!("({}, {})", l.at.0 + dx, l.at.1 + dy);
                self.edit(&[Seg::Field("poses"), Seg::Key(&pose), Seg::Index(self.layer), Seg::Field("at")], &at);
            }
            Act::NextClip => self.clip += 1,
            Act::Close => self.open = false,
        }
    }
}

/// While it's open the editor has the keyboard and the wheel.
pub(crate) fn capture(
    mut ed: ResMut<Editor>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mine: ResMut<EditorKeys>,
    mut scroll: ResMut<AccumulatedMouseScroll>,
    mut taken: ResMut<KeyboardTaken>,
) {
    taken.0 = ed.open;
    if !ed.open {
        return;
    }
    mine.0 = keys.clone();
    keys.reset_all();
    ed.scroll -= match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y * 24.0,
        MouseScrollUnit::Pixel => scroll.delta.y,
    };
    scroll.delta = Vec2::ZERO;
}

#[derive(Component)]
struct Root;

#[derive(Component)]
struct EdButton(Act);

#[derive(Component)]
pub(crate) struct Canvas;

#[derive(Component)]
struct Preview;

#[derive(Component)]
struct Status;

#[derive(Component)]
struct ItemList;

#[derive(Component)]
struct ChannelText(usize);

#[derive(Component)]
struct PointText;

fn buttons(mut ed: ResMut<Editor>, keys: Res<EditorKeys>, clicks: Query<(&Interaction, &EdButton), Changed<Interaction>>) {
    let shift = keys.0.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    for (i, b) in &clicks {
        if *i == Interaction::Pressed {
            ed.act(&b.0, shift);
        }
    }
}

fn keys(mut ed: ResMut<Editor>, keys: Res<EditorKeys>) {
    if !ed.open {
        return;
    }
    let k = &keys.0;
    let cmd = k.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]);
    let shift = k.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if k.any_just_pressed([KeyCode::Escape, KeyCode::KeyE]) {
        ed.open = false;
    }
    if cmd && k.just_pressed(KeyCode::KeyZ) {
        ed.act(if shift { &Act::Redo } else { &Act::Undo }, false);
    }
    if cmd && k.just_pressed(KeyCode::KeyY) {
        ed.act(&Act::Redo, false);
    }
    for (key, d) in [(KeyCode::ArrowLeft, (-1, 0)), (KeyCode::ArrowRight, (1, 0)), (KeyCode::ArrowUp, (0, -1)), (KeyCode::ArrowDown, (0, 1))] {
        if k.just_pressed(key) {
            ed.act(&Act::Nudge(d.0, d.1), false);
        }
    }
}

/// The mouse on the canvas.
fn paint(mut ed: ResMut<Editor>, mouse: Res<ButtonInput<MouseButton>>, canvas: Query<&RelativeCursorPosition, With<Canvas>>) {
    if !ed.open {
        return;
    }
    let Some((w, h)) = view_size(&ed) else { return };
    let over = canvas.single().ok().filter(|c| c.cursor_over()).and_then(|c| c.normalized);
    let at = over.map(|n| (((n.x + 0.5) * w as f32).floor() as i32, ((n.y + 0.5) * h as f32).floor() as i32));
    if at != ed.hover {
        ed.hover = at;
    }
    if let Some(at) = at {
        if mouse.just_pressed(MouseButton::Right) {
            ed.apply(at, true, true);
        }
        if mouse.just_pressed(MouseButton::Left) {
            let before = ed.text.clone();
            ed.stroke = Some((before, None));
        }
        if mouse.pressed(MouseButton::Left)
            && let Some((_, last)) = ed.stroke.clone()
            && last != Some(at)
        {
            ed.apply(at, last.is_none(), false);
            if let Some(s) = ed.stroke.as_mut() {
                s.1 = Some(at);
            }
        }
    }
    // The stroke ends: one undo for all of it, and saved.
    if !mouse.pressed(MouseButton::Left)
        && let Some((before, _)) = ed.stroke.take()
        && before != ed.text
    {
        ed.undo.push(before);
        ed.redo.clear();
        ed.save();
    }
}

/// Someone else wrote the file (a person, the model): take it up, undoably.
fn reload(mut ed: ResMut<Editor>) {
    if !ed.open {
        return;
    }
    if ed.files.is_empty() {
        ed.find_files();
        ed.open_file(0);
    }
    let changed = ed.watch.as_mut().is_some_and(|w| w.changed());
    if !changed || ed.stroke.is_some() {
        return;
    }
    let Some(text) = ed.path().and_then(|p| std::fs::read_to_string(p).ok()) else { return };
    if text != ed.written && text != ed.text {
        let old = std::mem::replace(&mut ed.text, text.clone());
        ed.undo.push(old);
        ed.written = text;
        ed.reparse();
        ed.status = format!("{} changed on disk: reloaded", ed.name());
    }
}

impl Editor {
    /// The canvas's size in pixels of the thing shown, and the file open.
    pub(crate) fn canvas(&self) -> Option<((u32, u32), PathBuf)> {
        Some((view_size(self)?, self.path()?.clone()))
    }
}

/// The canvas's size in pixels of the thing shown.
fn view_size(ed: &Editor) -> Option<(u32, u32)> {
    let f = ed.parsed.as_ref()?;
    match ed.item()? {
        (Kind::Part, n) => {
            let p = f.parts.get(&n)?;
            Some((p.rows.first().map_or(1, |r| r.chars().count()) as u32, p.rows.len() as u32))
        }
        _ => Some(f.size),
    }
}

/// How big each pixel is drawn.
fn scale(w: u32, h: u32) -> u32 {
    (420 / w.max(h).max(1)).clamp(4, 40)
}

const LABEL: f32 = 12.0;
const IDLE: Color = Color::srgba(0.25, 0.25, 0.3, 0.9);
const PICKED: Color = Color::srgba(0.55, 0.45, 0.15, 0.95);

fn text(t: impl Into<String>, size: f32) -> impl Bundle {
    (Text::new(t), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(Color::WHITE))
}

fn button(p: &mut ChildSpawnerCommands, label: &str, act: Act) {
    p.spawn((Button, EdButton(act), Node { padding: UiRect::axes(px(6), px(2)), ..default() }, BackgroundColor(IDLE))).with_children(|b| {
        b.spawn(text(label, LABEL));
    });
}

fn row(p: &mut ChildSpawnerCommands, width: f32, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(3), row_gap: px(3), max_width: px(width), ..default() })
        .with_children(f);
}

fn heading(p: &mut ChildSpawnerCommands, t: &str) {
    p.spawn((Text::new(t), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(0.75, 0.75, 0.8))));
}

/// (Re)build the editor's UI when what it lists has changed.
fn build_ui(mut commands: Commands, mut ed: ResMut<Editor>, roots: Query<Entity, With<Root>>) {
    let shape = if ed.open {
        let f = ed.parsed.as_ref();
        format!(
            "{}|{}|{:?}|{}|{:?}|{}",
            ed.file,
            ed.items.len(),
            f.map(|f| f.palette.keys().collect::<String>()),
            ed.sel,
            ed.layers().iter().map(|l| (&l.part, l.at)).collect::<Vec<_>>(),
            view_size(&ed).map_or(0, |(w, h)| w * 1000 + h)
        )
    } else {
        String::new()
    };
    if shape == ed.shape {
        return;
    }
    ed.shape = shape;
    ed.redraw = true;
    for r in &roots {
        commands.entity(r).despawn();
    }
    if !ed.open {
        return;
    }
    let (w, h) = view_size(&ed).unwrap_or((16, 16));
    let s = scale(w, h) as f32;
    let files: Vec<String> = ed.files.iter().map(|p| p.file_stem().unwrap_or_default().to_string_lossy().into_owned()).collect();
    let items = ed.items.clone();
    let palette: Vec<(char, [u8; 4])> = ed.parsed.as_ref().map(|f| f.palette.iter().map(|(c, v)| (*c, v.rgba())).collect()).unwrap_or_default();
    let layers = ed.layers();
    let art_size = ed.parsed.as_ref().map_or((16, 16), |f| f.size);
    commands
        .spawn((
            Root,
            Interaction::default(),
            Node {
                position_type: PositionType::Absolute,
                top: px(64),
                right: px(8),
                flex_direction: FlexDirection::Row,
                column_gap: px(10),
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.07, 0.97)),
        ))
        .with_children(|p| {
            // Files, and everything in the one open.
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(4), width: px(180), ..default() }).with_children(|c| {
                c.spawn((Text::new("ART EDITOR"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.3))));
                row(c, 180.0, |r| {
                    for (i, f) in files.iter().enumerate() {
                        button(r, f, Act::File(i));
                    }
                });
                heading(c, "In it");
                c.spawn((
                    ItemList,
                    ScrollPosition::default(),
                    Node { flex_direction: FlexDirection::Column, row_gap: px(2), height: px(430), overflow: Overflow::scroll_y(), ..default() },
                ))
                .with_children(|l| {
                    for (i, (kind, name)) in items.iter().enumerate() {
                        button(l, &format!("{} {name}", kind.word()), Act::Item(i));
                    }
                });
            });
            // Tools, the canvas, what's going on.
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(5), ..default() }).with_children(|c| {
                row(c, 460.0, |r| {
                    for (t, l) in [(Tool::Pencil, "Pencil"), (Tool::Erase, "Eraser"), (Tool::Fill, "Fill"), (Tool::Pick, "Pick"), (Tool::Point, "Point")] {
                        button(r, l, Act::Tool(t));
                    }
                    r.spawn((Button, EdButton(Act::NextPoint), Node { padding: UiRect::axes(px(6), px(2)), ..default() }, BackgroundColor(IDLE)))
                        .with_children(|b| {
                            b.spawn((PointText, Text::new("hand"), TextFont { font_size: FontSize::Px(LABEL), ..default() }, TextColor(Color::srgb(1.0, 0.9, 0.4))));
                        });
                    button(r, "Onion", Act::Onion);
                    button(r, "Undo", Act::Undo);
                    button(r, "Redo", Act::Redo);
                    button(r, "Close", Act::Close);
                });
                c.spawn((
                    Canvas,
                    Interaction::default(),
                    RelativeCursorPosition::default(),
                    ImageNode::default(),
                    Node { width: px(w as f32 * s), height: px(h as f32 * s), ..default() },
                ));
                c.spawn((Status, Text::new(""), TextFont { font_size: FontSize::Px(LABEL), ..default() }, TextColor(Color::srgb(0.8, 0.85, 0.9)), Node { max_width: px(460.0), ..default() }));
            });
            // Colours, layers, a clip playing.
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(5), width: px(200), ..default() }).with_children(|c| {
                heading(c, "Palette");
                row(c, 200.0, |r| {
                    for (ch, rgba) in &palette {
                        let bg = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3].max(60));
                        let light = (rgba[0] as u32 + rgba[1] as u32 + rgba[2] as u32) > 380;
                        r.spawn((
                            Button,
                            EdButton(Act::Color(*ch)),
                            Node { width: px(22), height: px(22), border: UiRect::all(px(2)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                            BackgroundColor(bg),
                            BorderColor::all(Color::BLACK),
                        ))
                        .with_children(|b| {
                            b.spawn((Text::new(ch.to_string()), TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(if light { Color::BLACK } else { Color::WHITE })));
                        });
                    }
                });
                for (i, n) in ["R", "G", "B"].iter().enumerate() {
                    row(c, 200.0, |r| {
                        r.spawn((ChannelText(i), Text::new(n.to_string()), TextFont { font_size: FontSize::Px(LABEL), ..default() }, TextColor(Color::WHITE), Node { width: px(52), ..default() }));
                        button(r, "-", Act::Channel(i, -1));
                        button(r, "+", Act::Channel(i, 1));
                    });
                }
                row(c, 200.0, |r| button(r, "New colour", Act::NewColor));
                if !layers.is_empty() {
                    heading(c, "Layers (arrows move)");
                    for (i, l) in layers.iter().enumerate() {
                        let mark = if l.flip { " (flipped)" } else { "" };
                        row(c, 200.0, |r| button(r, &format!("{} {} at {},{}{mark}", i, l.part, l.at.0, l.at.1), Act::Layer(i)));
                    }
                }
                heading(c, "Playing");
                row(c, 200.0, |r| button(r, "Next clip", Act::NextClip));
                c.spawn((Preview, ImageNode::default(), Node { width: px(art_size.0 as f32 * 4.0), height: px(art_size.1 as f32 * 4.0), ..default() }));
            });
        });
}

fn image(p: &Pixels) -> Image {
    Image::new(
        Extent3d { width: p.w.max(1), height: p.h.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if p.rgba.is_empty() { vec![0; 4] } else { p.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// The canvas drawn big: a checkerboard, the onion skin, the pixels, the
/// grid, the points.
fn draw_canvas(ed: &Editor) -> Option<Pixels> {
    let f = ed.parsed.as_ref()?;
    let art = ed.art.as_ref()?;
    let (kind, name) = ed.item()?;
    let (w, h) = view_size(ed)?;
    let s = scale(w, h);
    // What's shown, pixel by pixel.
    let mut shown = Pixels::new(w, h);
    let raw = |rows: &Vec<String>, into: &mut Pixels| {
        for (y, r) in rows.iter().enumerate() {
            for (x, c) in r.chars().enumerate() {
                if let Some(col) = f.palette.get(&c) {
                    into.set(x as i32, y as i32, col.rgba());
                }
            }
        }
    };
    let index = art.index(&name);
    match kind {
        Kind::Frame => raw(f.frames.get(&name)?, &mut shown),
        Kind::Part => raw(&f.parts.get(&name)?.rows, &mut shown),
        _ => shown = art.frames.get(index?)?.clone(),
    }
    // The frame before it in its clip, faint.
    let onion = (ed.onion && kind != Kind::Part)
        .then(|| {
            let clip = f.clips.values().find(|c| c.frames.contains(&name))?;
            let i = clip.frames.iter().position(|n| *n == name)?;
            let prev = &clip.frames[(i + clip.frames.len() - 1) % clip.frames.len()];
            (*prev != name).then(|| art.index(prev)).flatten().map(|k| art.frames[k].clone())
        })
        .flatten();
    let mut marks: Vec<((i32, i32), [u8; 4])> = Vec::new();
    let mut boxed = None;
    match kind {
        Kind::Part => {
            let p = f.parts.get(&name)?;
            marks.push((p.pivot, [80, 220, 255, 255]));
            marks.extend(p.points.values().map(|&pt| (pt, [255, 220, 60, 255])));
        }
        _ => {
            if let Some(i) = index {
                marks.extend(art.anchors.values().filter_map(|m| m.get(&i)).map(|&pt| (pt, [255, 220, 60, 255])));
            }
            marks.push(((f.feet.0.floor() as i32, f.feet.1 as i32 - 1), [255, 60, 60, 255]));
            if let Some(t) = ed.target()
                && let (Some(part), Some(Placed { at, pivot, flip })) = (&t.part, t.layer)
                && let Some(p) = f.parts.get(part)
            {
                let pw = p.rows.first().map_or(0, |r| r.chars().count()) as i32;
                let x0 = if flip { at.0 + pivot.0 - (pw - 1) } else { at.0 - pivot.0 };
                boxed = Some((x0, at.1 - pivot.1, pw, p.rows.len() as i32));
            }
        }
    }
    let mut out = Pixels::new(w * s, h * s);
    let grid = s >= 8;
    for py in 0..h * s {
        for px in 0..w * s {
            let (x, y) = ((px / s) as i32, (py / s) as i32);
            let checker = if (x + y) % 2 == 0 { [58, 58, 66] } else { [70, 70, 80] };
            let mut c = [checker[0] as f32, checker[1] as f32, checker[2] as f32];
            let mut over = |rgba: [u8; 4], k: f32| {
                let a = rgba[3] as f32 / 255.0 * k;
                for i in 0..3 {
                    c[i] = c[i] * (1.0 - a) + rgba[i] as f32 * a;
                }
            };
            if let Some(o) = &onion {
                over(o.get(x, y), 0.3);
            }
            over(shown.get(x, y), 1.0);
            if grid && (px % s == 0 || py % s == 0) {
                over([20, 20, 24, 255], 0.35);
            }
            let (ix, iy) = (px % s, py % s);
            let edge = ix == 0 || iy == 0 || ix == s - 1 || iy == s - 1;
            let thick = ix <= 1 || iy <= 1 || ix >= s - 2 || iy >= s - 2;
            for &(pt, col) in &marks {
                if pt == (x, y) && thick {
                    over(col, 1.0);
                }
            }
            if let Some((bx, by, bw, bh)) = boxed {
                let inside = x >= bx && x < bx + bw && y >= by && y < by + bh;
                let border = (x == bx && ix == 0) || (x == bx + bw - 1 && ix == s - 1) || (y == by && iy == 0) || (y == by + bh - 1 && iy == s - 1);
                if inside && border && edge {
                    over([255, 240, 120, 255], 0.9);
                }
            }
            if ed.hover == Some((x, y)) && edge {
                over([255, 255, 255, 255], 0.7);
            }
            out.set(px as i32, py as i32, [c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        }
    }
    Some(out)
}

/// The texts that change: a channel's value, the status, the point's name.
type Labels<'a> = (&'a mut Text, Option<&'a ChannelText>, Has<Status>);
type LabelOnly = Or<(With<Status>, With<ChannelText>, With<PointText>)>;

#[allow(clippy::too_many_arguments)]
fn show(
    time: Res<Time<Real>>,
    mut ed: ResMut<Editor>,
    mut images: ResMut<Assets<Image>>,
    mut canvas: Query<&mut ImageNode, (With<Canvas>, Without<Preview>)>,
    mut preview: Query<&mut ImageNode, (With<Preview>, Without<Canvas>)>,
    mut labels: Query<Labels, LabelOnly>,
    mut buttons: Query<(&Interaction, &EdButton, &mut BackgroundColor, Option<&mut BorderColor>)>,
    mut lists: Query<&mut ScrollPosition, With<ItemList>>,
    mut last_hover: Local<Option<(i32, i32)>>,
    mut last_frame: Local<Option<(usize, usize)>>,
) {
    if !ed.open {
        return;
    }
    if ed.hover != *last_hover {
        *last_hover = ed.hover;
        ed.redraw = true;
    }
    if ed.redraw
        && let Some(pix) = draw_canvas(&ed)
        && let Ok(mut node) = canvas.single_mut()
    {
        node.image = images.add(image(&pix));
        ed.redraw = false;
    }
    // A clip playing.
    ed.anim += time.delta_secs();
    if let Some(art) = &ed.art
        && !art.clips.is_empty()
        && let Some(clip) = art.clips.values().nth(ed.clip % art.clips.len())
        && let Ok(mut node) = preview.single_mut()
    {
        let k = clip.frames[((ed.anim * clip.fps) as usize) % clip.frames.len().max(1)];
        // (A new image when the frame changes, or the art did.)
        let key = (k, ed.undo.len() + ed.redo.len() * 7919 + ed.text.len());
        if *last_frame != Some(key) || node.image == Handle::default() {
            *last_frame = Some(key);
            node.image = images.add(image(&art.frames[k]));
        }
    }
    let names = ed.point_names();
    let pname = ed.point_name();
    let clip_name = ed.art.as_ref().and_then(|a| a.clips.keys().nth(ed.clip % a.clips.len().max(1)).cloned()).unwrap_or_default();
    let col = ed.parsed.as_ref().and_then(|f| f.palette.get(&ed.color)).map(|c| c.rgba()).unwrap_or([0; 4]);
    for (mut t, ch, status) in &mut labels {
        t.0 = match ch {
            Some(ch) => format!("{} {}", ["R", "G", "B"][ch.0], col[ch.0]),
            None if status => {
                let at = ed.hover.map(|(x, y)| format!("{x},{y}  ")).unwrap_or_default();
                let what = ed.item().map(|(k, n)| format!("{} {n}", k.word())).unwrap_or_default();
                format!("{at}{} | {what} | colour '{}' | playing {clip_name}\n{}", ed.name(), ed.color, ed.error.as_deref().unwrap_or(&ed.status))
            }
            None => format!("{pname} ({}/{})", ed.point % names.len().max(1) + 1, names.len()),
        };
    }
    let (sel, tool, color, layer, onion, file) = (ed.sel, ed.tool, ed.color, ed.layer, ed.onion, ed.file);
    for (i, b, mut bg, border) in &mut buttons {
        if let Act::Color(c) = b.0 {
            if let Some(mut border) = border {
                *border = BorderColor::all(if c == color { Color::WHITE } else if *i == Interaction::Hovered { Color::srgb(0.6, 0.6, 0.6) } else { Color::BLACK });
            }
            continue;
        }
        let on = match b.0 {
            Act::Item(k) => k == sel,
            Act::Tool(t) => t == tool,
            Act::Layer(k) => k == layer,
            Act::Onion => onion,
            Act::File(k) => k == file,
            _ => false,
        };
        bg.0 = match i {
            Interaction::Pressed => Color::srgba(0.6, 0.5, 0.2, 0.95),
            Interaction::Hovered => Color::srgba(0.35, 0.35, 0.42, 0.95),
            Interaction::None if on => PICKED,
            Interaction::None => IDLE,
        };
    }
    for mut s in &mut lists {
        s.0.y = (s.0.y + ed.scroll).max(0.0);
    }
    ed.scroll = 0.0;
}
