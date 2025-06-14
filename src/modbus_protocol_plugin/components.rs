use bevy::prelude::*;
use chrono::{DateTime, Utc};
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

/// Represents a Modbus read request emitted by ECS.
#[derive(Debug, Clone)]
pub struct ModbusRequest {
    pub external_id: String,
    pub register_map_key: String,
}

impl ModbusRequest {
    pub fn new(external_id: String, register_map_key: String) -> Self {
        Self { external_id, register_map_key }
    }
}

/// Represents a Modbus read response pushed back into ECS.
#[derive(Debug, Clone)]
pub struct ModbusResponse {
    pub external_id: String,
    pub power_kw: f32,
    pub energy_kwh: f64,
    pub timestamp: DateTime<Utc>,
}

impl ModbusResponse {
    pub fn new(external_id: String, power_kw: f32, energy_kwh: f64, timestamp: DateTime<Utc>) -> Self {
        Self { external_id, power_kw, energy_kwh, timestamp }
    }
}
