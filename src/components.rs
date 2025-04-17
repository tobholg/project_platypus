use bevy::prelude::*;

/// simple flag
#[derive(Component)]
pub struct Player {
    pub grounded: bool,
}

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Exhaust {
    pub life: f32,
}

#[derive(Component)]
pub struct TileSprite {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last:  usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);