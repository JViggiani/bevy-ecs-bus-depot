use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::types::{EMeteringDataSource}; 
use chrono::{DateTime, Utc};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct ExternalId(pub String);

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct AssetInfo {
    pub make: String,
    pub model: String,
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize, Default)]
pub struct CurrentMeterReading {
    pub power_kw: f32,
    pub energy_kwh: f64,
    #[reflect(ignore)] 
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>, 
}


#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize, Default)]
pub struct TargetPowerSetpointKw(pub f32);

#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize, Default)]
pub struct LastAppliedSetpointKw(pub f32);


#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeteringSourceDetails {
    Modbus {
        ip: String,
        port: u16,
        unit_id: u8,
        poll_interval_ms: u32,
        register_map_key: String, 
    },
    Ocpp {
    },
    InternalCalculation {
    }
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct MeteringSource {
    pub source_type: EMeteringDataSource, 
    pub details: Option<MeteringSourceDetails>, 
}
