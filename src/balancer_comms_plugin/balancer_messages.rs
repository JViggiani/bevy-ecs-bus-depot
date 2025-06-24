use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// External message formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancerSetpointMessage {
    pub external_id: String,
    pub target_power_kw: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancerMeteringMessage {
    pub external_id: String,
    pub power_kw: f32,
    pub energy_kwh: f64,
    pub timestamp: DateTime<Utc>,
}
