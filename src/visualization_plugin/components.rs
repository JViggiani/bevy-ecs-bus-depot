use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct AssetLabel;

#[derive(Component)]
pub struct Orchestrator;

#[derive(Component)]
pub struct ConnectionLine;

#[derive(Component)]
pub struct Balancer;

#[derive(Component)]
pub struct Visualized;
