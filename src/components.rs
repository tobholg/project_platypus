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

/* ===========================================================
   enemies
   =========================================================== */
#[derive(Component)]
pub struct Enemy {
    pub grounded: bool,
    pub hp: i32,
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
   
   /* re‑use existing Exhaust component for debris?  
      → we add a separate one so we can tune lifetime/colors */
   #[derive(Component)]
   pub struct Debris {
       pub life: f32,
   }