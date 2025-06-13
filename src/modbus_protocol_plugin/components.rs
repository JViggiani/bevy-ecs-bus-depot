use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct ModbusControlConfig {
    pub ip: String,
    pub port: u16,
    pub unit_id: u8,
}

#[derive(Component, Debug, Copy, Clone, Reflect, Default)]
#[reflect(Component, Default)]
pub struct ModbusAssetLastPoll(pub f32);
