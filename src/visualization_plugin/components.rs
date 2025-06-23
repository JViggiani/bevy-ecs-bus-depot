use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct AssetLabel;

#[derive(Component)]
pub struct ConnectionLine(pub Vec3, pub Vec3);

#[derive(Component)]
pub struct Visualized;

#[derive(Component)]
pub struct OrchestratorVisuals;

#[derive(Component)]
pub struct BalancerVisuals;

#[derive(Component)]
pub struct TextColor(pub Color);

#[derive(Component)]
pub struct TextFont {
    pub font_size: f32,
}

#[derive(Component)]
pub struct Text2d;

