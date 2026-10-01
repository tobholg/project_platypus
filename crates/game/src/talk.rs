//! Talking to villagers (DESIGN §13 item 6).
//!
//! Walk up to one and it stops and says something: the news (what's
//! happened lately: `events.rs`), then its `lines`, one after another, in a
//! speech bubble over its head (the next time you come by, it goes on
//! where it left off).
//!
//! Right-click one (near) and its panel opens beside the pack: what it does
//! for you, for gold. The smith and the merchant sell (click: one; Shift:
//! ten); the healer heals you whole and sells potions; after a raid the
//! guide takes gold to have the village mended by morning; the merchant buys
//! (drop a stack on its slot: half what anyone here sells it for, a gold
//! apiece for what nobody sells, nothing for plain blocks); the guide lists
//! its tips. Walk off, or close the pack, and it shuts.
//!
//! The panel's clicks are `Trade` messages (so a scenario trades as a
//! player does).

use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use bevy::text::TextBounds;
use std::collections::HashMap;

use crate::actors::player::LocalPlayer;
use crate::actors::villager::{Routine, TALK_NEAR, Villager};
use crate::actors::{Health, Kinematics};
use crate::camera::CursorWorld;
use crate::gold::Gold;
use crate::hands::icons::Icons;
use crate::hands::items::{Inventory, ItemId, Items, Stack, Use};
use crate::hands::ui::{Held, UNDER_PACK};
use crate::hands::{DevTools, InventoryOpen};
use crate::sound::PlaySound;

pub struct TalkPlugin;

impl Plugin for TalkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shop>()
            .add_message::<Trade>()
            .add_systems(Startup, spawn_panel)
            .add_systems(Update, (bubbles, open, close, build_panel, clicks, sell_slot, trade, show).chain());
    }
}

/// A line stays up this long (seconds).
const LINE_SECS: f32 = 4.5;
/// The bubble: its text size (drawn big, scaled down: crisp), how much
/// smaller it's drawn, how wide it wraps, how far over the head.
const BUBBLE_FONT: f32 = 22.0;
const BUBBLE_SCALE: f32 = 0.25;
const BUBBLE_WIDTH: f32 = 230.0;
const BUBBLE_OVER: f32 = 5.0;
/// Shift-click buys this many.
const MANY: u32 = 10;
/// The gold colour of prices.
const GOLDEN: Color = Color::srgb(1.0, 0.84, 0.3);

/// The villager whose panel is open, what it offers, and what just
/// happened (the panel's last line).
#[derive(Resource, Default)]
pub struct Shop {
    pub with: Option<Entity>,
    pub offers: Vec<Offer>,
    pub buys: bool,
    note: String,
}

/// One thing a villager does for gold.
#[derive(Clone, Debug)]
pub enum Offer {
    /// An item, gold each.
    Buy { item: ItemId, price: u32 },
    /// Health back, all of it.
    Heal(u32),
    /// The village mended by morning (the guide's, after a raid).
    Mend(u32),
}

/// What the player does at the panel.
#[derive(Message, Clone, Copy, Debug)]
pub enum Trade {
    /// Take up an offer, `n` times (a heal: once).
    Take { offer: usize, n: u32 },
    /// Sell the stack in the mouse's grip.
    Sell,
}

/// A villager's bubble and where its talk is up to.
struct Said {
    bubble: Option<Entity>,
    line: usize,
    next: f32,
}

#[derive(Component)]
struct Bubble;

#[derive(Component)]
struct ShopRoot;
/// The panel's changing part (rebuilt for each villager).
#[derive(Component)]
struct ShopBody;
#[derive(Component)]
struct ShopTitle;
#[derive(Component)]
struct ShopPurse;
#[derive(Component)]
struct ShopNote;
/// An offer's row (click: take it), and its price.
#[derive(Component)]
struct OfferRow(usize);
#[derive(Component)]
struct OfferPrice(usize);
/// The merchant's slot: drop a stack on it to sell it.
#[derive(Component)]
struct SellSlot;
#[derive(Component)]
struct SellText;

/// The villager nearest a player says its lines, one at a time, over its
/// head (one at a time: neighbours don't talk over each other).
fn bubbles(
    mut commands: Commands,
    time: Res<Time>,
    mut said: Local<HashMap<Entity, Said>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    folk: Query<(Entity, &Villager, &Kinematics, &Routine)>,
    news: Res<crate::events::News>,
    mut shown: Query<(&mut Text2d, &mut Transform), With<Bubble>>,
) {
    let now = time.elapsed_secs();
    let speaker = player.single().ok().and_then(|pk| {
        let d = |k: &Kinematics| k.body.pos.distance_squared(pk.body.pos);
        folk.iter().filter(|(.., r)| r.talking).min_by(|a, b| d(a.2).total_cmp(&d(b.2))).map(|(e, ..)| e)
    });
    for (e, v, k, _) in &folk {
        let s = said.entry(e).or_insert(Said { bubble: None, line: 0, next: 0.0 });
        let at = (k.body.pos + Vec2::new(0.0, k.body.half.y + BUBBLE_OVER)).extend(30.0);
        // (The news first: what's happened lately.)
        let lines: Vec<&String> = news.0.iter().take(1).chain(&v.lines).collect();
        if speaker != Some(e) || lines.is_empty() {
            if let Some(b) = s.bubble.take() {
                commands.entity(b).despawn();
                // (Next time, the next line.)
                s.line += 1;
            }
            continue;
        }
        let line = |i: usize| lines[i % lines.len()].clone();
        match s.bubble.and_then(|b| shown.get_mut(b).ok()) {
            Some((mut text, mut tf)) => {
                tf.translation = at;
                if now >= s.next {
                    s.line += 1;
                    s.next = now + LINE_SECS;
                    text.0 = line(s.line);
                }
            }
            None => {
                s.next = now + LINE_SECS;
                let b = commands
                    .spawn((
                        Name::new("Speech"),
                        Bubble,
                        Text2d::new(line(s.line)),
                        TextFont { font_size: FontSize::Px(BUBBLE_FONT), ..default() },
                        TextColor(Color::srgb(1.0, 0.97, 0.88)),
                        TextLayout::justify(Justify::Center),
                        TextBounds::new_horizontal(BUBBLE_WIDTH),
                        Text2dShadow { offset: Vec2::new(2.0, -2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
                        Anchor::BOTTOM_CENTER,
                        Transform::from_translation(at).with_scale(Vec3::splat(BUBBLE_SCALE)),
                    ))
                    .id();
                s.bubble = Some(b);
            }
        }
    }
    // The gone: their bubbles too.
    said.retain(|e, s| {
        let here = folk.contains(*e);
        if !here && let Some(b) = s.bubble {
            commands.entity(b).despawn();
        }
        here
    });
}

/// Right-click a villager near you: its panel (and the pack) open.
#[allow(clippy::too_many_arguments)]
fn open(
    mouse: Res<ButtonInput<MouseButton>>,
    dev: Res<DevTools>,
    cursor: Res<CursorWorld>,
    items: Option<Res<Items>>,
    mut shop: ResMut<Shop>,
    mut inv_open: ResMut<InventoryOpen>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    village: Res<crate::events::Village>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    folk: Query<(Entity, &Villager, &Kinematics)>,
) {
    if dev.0 || !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(at), Some(items), Ok(pk)) = (cursor.0, items, player.single()) else { return };
    let Some((e, v, _)) = folk
        .iter()
        .filter(|(_, _, k)| ((k.body.pos - at).abs() - k.body.half).max_element() <= 3.0 && k.body.pos.distance(pk.body.pos) <= TALK_NEAR)
        .min_by(|a, b| a.2.body.pos.distance(at).total_cmp(&b.2.body.pos.distance(at)))
    else {
        return;
    };
    *shop = open_for(e, v, &items, village.missing > 0 && !village.paid);
    chests.open = None;
    inv_open.0 = true;
}

/// What `v` offers (`mend`: the village wants mending, unpaid for: the
/// guide takes gold for it).
pub fn open_for(e: Entity, v: &Villager, items: &Items, mend: bool) -> Shop {
    let mut offers: Vec<Offer> = v.heals.map(Offer::Heal).into_iter().collect();
    if mend && v.role == "guide" {
        offers.push(Offer::Mend(crate::events::MEND_PRICE));
    }
    for (name, price) in &v.sells {
        match items.id(name) {
            Some(item) => offers.push(Offer::Buy { item, price: *price }),
            None => warn!("{} sells {name}: no such item", v.role),
        }
    }
    Shop { with: Some(e), offers, buys: v.buys, note: String::new() }
}

/// The panel shuts when the pack does, or you walk off, or it runs.
fn close(mut shop: ResMut<Shop>, inv_open: Res<InventoryOpen>, player: Query<&Kinematics, With<LocalPlayer>>, folk: Query<(&Kinematics, &Routine), With<Villager>>) {
    let Some(e) = shop.with else { return };
    let near = match (player.single(), folk.get(e)) {
        (Ok(pk), Ok((k, r))) => !r.fleeing && k.body.pos.distance(pk.body.pos) <= TALK_NEAR * 1.5,
        _ => false,
    };
    if !inv_open.0 || !near {
        shop.with = None;
    }
}

fn text(s: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color))
}

fn spawn_panel(mut commands: Commands) {
    commands
        .spawn((ShopRoot, Visibility::Hidden, Node { position_type: PositionType::Absolute, top: px(UNDER_PACK), left: px(10), ..default() }))
        .with_children(|root| {
            root.spawn((
                Interaction::None,
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), padding: UiRect::all(px(10)), min_width: px(300), max_width: px(460), ..default() },
                BackgroundColor(Color::srgba(0.1, 0.07, 0.04, 0.78)),
            ))
            .with_children(|panel| {
                panel.spawn(Node { column_gap: px(16), justify_content: JustifyContent::SpaceBetween, ..default() }).with_children(|top| {
                    top.spawn((ShopTitle, text("", 16.0, Color::WHITE)));
                    top.spawn((ShopPurse, text("", 14.0, GOLDEN)));
                });
                panel.spawn((ShopBody, Node { flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }));
                panel.spawn((ShopNote, text("", 13.0, Color::srgb(0.95, 0.9, 0.8))));
            });
        });
}

/// The panel's rows, for the villager whose it is (when that changes).
#[allow(clippy::too_many_arguments)]
fn build_panel(
    mut commands: Commands,
    shop: Res<Shop>,
    mut last: Local<Option<Entity>>,
    items: Option<Res<Items>>,
    icons: Option<Res<Icons>>,
    folk: Query<&Villager>,
    body: Single<Entity, With<ShopBody>>,
    mut title: Single<&mut Text, With<ShopTitle>>,
) {
    if *last == shop.with {
        return;
    }
    *last = shop.with;
    commands.entity(*body).despawn_children();
    let (Some(e), Some(items)) = (shop.with, items) else { return };
    let Ok(v) = folk.get(e) else { return };
    title.0 = capital(&v.role);
    let small = Color::srgb(0.8, 0.8, 0.86);
    commands.entity(*body).with_children(|b| {
        for (i, offer) in shop.offers.iter().enumerate() {
            b.spawn((
                Button,
                OfferRow(i),
                Node { column_gap: px(10), align_items: AlignItems::Center, padding: UiRect::axes(px(6), px(2)), border_radius: BorderRadius::all(px(5)), ..default() },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
            ))
            .with_children(|row| {
                let (icon, name) = match offer {
                    Offer::Buy { item, .. } => {
                        let def = items.def(*item);
                        let (r, g, bl) = def.color;
                        let img = icons.as_ref().and_then(|ic| ic.get(*item)).cloned();
                        let bg = if img.is_some() { Color::NONE } else { Color::srgb_u8(r, g, bl) };
                        (Some((img.unwrap_or_default(), bg)), def.name.clone())
                    }
                    Offer::Heal(_) => (None, "Heal all your wounds".to_string()),
                    Offer::Mend(_) => (None, "Mend the village by morning".to_string()),
                };
                let square = Node { width: px(32), height: px(32), ..default() };
                match icon {
                    // (No icon: the item's colour, a square of it.)
                    Some((img, bg)) if img == Handle::default() => {
                        row.spawn((square, BackgroundColor(bg)));
                    }
                    Some((img, bg)) => {
                        row.spawn((ImageNode::new(img), square, BackgroundColor(bg)));
                    }
                    None => {
                        let (mark, color) = if matches!(offer, Offer::Mend(_)) { ("#", Color::srgb(0.75, 0.55, 0.3)) } else { ("♥", Color::srgb(0.95, 0.3, 0.35)) };
                        row.spawn((text(mark, 22.0, color), Node { width: px(32), justify_content: JustifyContent::Center, ..default() }));
                    }
                }
                row.spawn((text(name, 14.0, Color::WHITE), Node { flex_grow: 1.0, ..default() }));
                row.spawn((OfferPrice(i), text("", 14.0, GOLDEN)));
            });
        }
        if shop.buys {
            b.spawn(Node { column_gap: px(10), align_items: AlignItems::Center, margin: UiRect::top(px(4)), ..default() }).with_children(|row| {
                row.spawn((
                    Button,
                    SellSlot,
                    Node { width: px(40), height: px(40), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(7)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BorderColor::all(GOLDEN),
                    BackgroundColor(Color::srgba(0.35, 0.26, 0.08, 0.8)),
                ))
                .with_child(text("$", 18.0, GOLDEN));
                row.spawn((SellText, text("", 13.0, small)));
            });
        }
        if shop.offers.is_empty() && !shop.buys {
            // (The guide: its tips.)
            for line in &v.lines {
                b.spawn((text(format!("• {line}"), 13.0, small), Node { max_width: px(440), ..default() }));
            }
        } else if !shop.offers.is_empty() {
            b.spawn(text("Click: buy one  |  Shift-click: ten", 12.0, Color::srgba(0.8, 0.8, 0.86, 0.7)));
        }
    });
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// A click on a row takes that offer.
fn clicks(keys: Res<ButtonInput<KeyCode>>, rows: Query<(&Interaction, &OfferRow), Changed<Interaction>>, mut out: MessageWriter<Trade>) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    for (i, row) in &rows {
        if *i == Interaction::Pressed {
            out.write(Trade::Take { offer: row.0, n: if shift { MANY } else { 1 } });
        }
    }
}

/// A stack let go of (or clicked down) over the merchant's slot is sold.
fn sell_slot(mouse: Res<ButtonInput<MouseButton>>, held: Res<Held>, slot: Query<&Interaction, With<SellSlot>>, mut out: MessageWriter<Trade>) {
    let over = slot.iter().any(|i| *i != Interaction::None);
    if over && held.stack.is_some() && (mouse.just_pressed(MouseButton::Left) || mouse.just_released(MouseButton::Left)) {
        out.write(Trade::Sell);
    }
}

/// What the merchant pays for a stack: half what anyone here sells one for
/// (more for finer gear), a gold apiece for what nobody sells, nothing for
/// plain blocks.
pub fn worth(items: &Items, folk: &[&Villager], s: &Stack) -> u32 {
    let def = items.def(s.item);
    let n = s.count / items.unit(s.item);
    let price = folk.iter().flat_map(|v| &v.sells).filter(|(id, _)| *id == def.id).map(|(_, p)| *p).min();
    let each = match price {
        Some(p) => (p / 2).max(1) * (1 + s.roll.rarity as u32),
        None if matches!(def.use_, Use::Block(_)) => 0,
        None => 1,
    };
    n * each
}

#[allow(clippy::too_many_arguments)]
fn trade(
    mut trades: MessageReader<Trade>,
    mut shop: ResMut<Shop>,
    items: Option<Res<Items>>,
    mut held: ResMut<Held>,
    mut village: ResMut<crate::events::Village>,
    folk: Query<&Villager>,
    mut player: Query<(&mut Gold, &mut Inventory, &mut Health), With<LocalPlayer>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let (Some(items), Ok((mut gold, mut inv, mut health))) = (items, player.single_mut()) else {
        trades.clear();
        return;
    };
    for t in trades.read() {
        if shop.with.is_none() {
            continue;
        }
        let paid = match *t {
            Trade::Take { offer, n } => match shop.offers.get(offer).cloned() {
                Some(Offer::Buy { item, price }) => {
                    let n = n.min(gold.0 / price.max(1));
                    let name = &items.def(item).name;
                    if n == 0 {
                        shop.note = format!("Not enough gold for {name}");
                        continue;
                    }
                    let unit = items.unit(item);
                    let left = inv.add(&items, Stack::new(item, n * unit));
                    let got = n - left.div_ceil(unit);
                    if got == 0 {
                        shop.note = "No room in your pack".into();
                        continue;
                    }
                    gold.0 -= got * price;
                    shop.note = if got == 1 { format!("Bought {name}") } else { format!("Bought {got} x {name}") };
                    true
                }
                Some(Offer::Heal(price)) => {
                    if health.hp >= health.max {
                        shop.note = "You're whole already".into();
                        continue;
                    }
                    if gold.0 < price {
                        shop.note = "Not enough gold".into();
                        continue;
                    }
                    gold.0 -= price;
                    health.hp = health.max;
                    shop.note = "Healed".into();
                    true
                }
                Some(Offer::Mend(price)) => {
                    if village.paid || village.missing == 0 {
                        shop.note = "It's in hand already".into();
                        continue;
                    }
                    if gold.0 < price {
                        shop.note = "Not enough gold".into();
                        continue;
                    }
                    gold.0 -= price;
                    village.paid = true;
                    shop.note = "It'll be mended by morning".into();
                    true
                }
                None => continue,
            },
            Trade::Sell => {
                let Some(s) = held.stack else { continue };
                if !shop.buys {
                    continue;
                }
                let all: Vec<&Villager> = folk.iter().collect();
                let v = worth(&items, &all, &s);
                let name = &items.def(s.item).name;
                if v == 0 {
                    shop.note = format!("No use for {name}");
                    continue;
                }
                held.stack = None;
                gold.0 = gold.0.saturating_add(v);
                shop.note = format!("Sold {name} for {v} gold");
                true
            }
        };
        if paid {
            sounds.write(PlaySound::here("gold").volume(0.7));
        }
    }
}

/// A price's text, and the texts that aren't the purse or the note.
type Price<'a> = (&'a OfferPrice, &'a mut Text, &'a mut TextColor);
type Lines = (Without<ShopPurse>, Without<ShopNote>);

/// The panel up (or not), the purse, the prices (red when out of reach),
/// the rows lit under the mouse, what the stack in hand would fetch.
#[allow(clippy::too_many_arguments)]
fn show(
    shop: Res<Shop>,
    items: Option<Res<Items>>,
    held: Res<Held>,
    folk: Query<&Villager>,
    player: Query<&Gold, With<LocalPlayer>>,
    mut root: Single<&mut Visibility, With<ShopRoot>>,
    mut purse: Single<&mut Text, (With<ShopPurse>, Without<ShopNote>)>,
    mut note: Single<&mut Text, (With<ShopNote>, Without<ShopPurse>)>,
    mut prices: Query<Price, (Lines, Without<SellText>)>,
    mut rows: Query<(&Interaction, &mut BackgroundColor), With<OfferRow>>,
    mut sell: Query<&mut Text, (With<SellText>, Lines, Without<OfferPrice>)>,
) {
    let vis = if shop.with.is_some() { Visibility::Visible } else { Visibility::Hidden };
    if **root != vis {
        **root = vis;
    }
    let Some(items) = items else { return };
    if shop.with.is_none() {
        return;
    }
    let gold = player.single().map_or(0, |g| g.0);
    purse.0 = format!("{gold} gold");
    if note.0 != shop.note {
        note.0 = shop.note.clone();
    }
    for (p, mut t, mut c) in &mut prices {
        let price = match shop.offers.get(p.0) {
            Some(Offer::Buy { price, .. }) | Some(Offer::Heal(price)) | Some(Offer::Mend(price)) => *price,
            None => continue,
        };
        t.0 = format!("{price} g");
        c.0 = if gold >= price { GOLDEN } else { Color::srgb(0.85, 0.35, 0.3) };
    }
    for (i, mut bg) in &mut rows {
        bg.0 = match i {
            Interaction::None => Color::srgba(1.0, 1.0, 1.0, 0.04),
            _ => Color::srgba(1.0, 0.9, 0.6, 0.16),
        };
    }
    for mut t in &mut sell {
        t.0 = match held.stack {
            Some(s) => {
                let all: Vec<&Villager> = folk.iter().collect();
                match worth(&items, &all, &s) {
                    0 => format!("{}: no use to me", items.def(s.item).name),
                    v => format!("{}: {v} gold", items.def(s.item).name),
                }
            }
            None => "Sell: drop a stack from your pack here".into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands::items::test_items;

    #[test]
    fn the_merchant_pays_half() {
        let items = test_items();
        let smith = Villager { sells: vec![("longsword".into(), 160), ("block:firebrick".into(), 3)], ..default() };
        let folk = [&smith];
        let sword = items.id("longsword").expect("longsword");
        assert_eq!(worth(&items, &folk, &Stack::new(sword, 1)), 80);
        let torch = items.id("torch").expect("torch");
        assert_eq!(worth(&items, &folk, &Stack::new(torch, 7)), 7, "a gold apiece for what nobody sells");
        let dirt = items.id("block:dirt").expect("dirt");
        assert_eq!(worth(&items, &folk, &Stack::new(dirt, 16 * 50)), 0, "plain blocks fetch nothing");
    }
}
