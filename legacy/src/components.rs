use bevy::prelude::*;

/* ===========================================================
   shared components
   =========================================================== */
#[derive(Component)]
pub struct Velocity(pub Vec2);

/* ===========================================================
   player
   =========================================================== */
#[derive(Component)]
pub struct Player {
    pub grounded: bool,
}

/* dash state --------------------------------------------------------- */
#[derive(Component)]
pub struct Dashing {
    pub remaining: f32,   // time left in seconds
    pub dir: f32,         // +1.0 right, −1.0 left
}

/* ========================================================
health and HUD
======================================================== */
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max:     f32,
    /// seconds since the player last took damage (for regen)
    pub last_damage: f32,
}

#[derive(Component)]
pub struct ToolbarText;

#[derive(Component)]
pub struct HealthBarFill;
   
/* ===========================================================
    inventory HUD slots
    =========================================================== */
#[derive(Component)]
pub struct InventorySlot(pub u8);   // 1 = pickaxe, 2 = gun, 3 = stone

#[derive(Component)]
pub struct Debris {
    pub life: f32,
}

/* ===========================================================
   enemies
   =========================================================== */
#[derive(Component)]
pub struct Enemy {
    pub grounded: bool,
    pub hp: i32,
    pub recoil: f32,
    /// seconds until the next swing is allowed
    pub attack_cooldown: f32,
    /// handle for the idle (walking / standing) sprite sheet
    pub idle_sheet: Handle<Image>,
    /// handle for the attack sprite sheet
    pub attack_sheet: Handle<Image>,
    /// set to `true` right after a swing begins; cleared once frame 4 lands
    pub hit_pending: bool,
}

/* tag added/removed every frame by update_active_tag_system */
#[derive(Component)]
pub struct Active;

/* ===========================================================
   animation helpers
   =========================================================== */
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last:  usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/* ===========================================================
   terrain helper components
   =========================================================== */
#[derive(Component)]
pub struct TileSprite {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct Highlight;

/* ===========================================================
   particles
   =========================================================== */
#[derive(Component)]
pub struct Exhaust {
    pub life: f32,
}

/* ========================================================
   inventory & weapons
   ======================================================== */
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HeldItem {
    Pickaxe,
    Gun,
    StoneBlock,
}

#[derive(Component)]
pub struct Inventory {
    pub selected: HeldItem,
}

#[derive(Component)]
pub struct Bullet {
    pub damage: f32,
    pub life:   f32,
}