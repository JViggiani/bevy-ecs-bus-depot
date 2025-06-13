use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::types::{EOcppVersion, EChargingRateUnit}; 
use chrono::{DateTime, Utc};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct OcppConfig {
    pub version: EOcppVersion,
    pub charge_point_id: String, 
}

#[derive(Component, Debug, Clone, Reflect, Default, Serialize, Deserialize)]
#[reflect(Component, Default, Serialize, Deserialize)]
pub struct OcppConnectionState {
    pub is_connected: bool,
    #[reflect(ignore)] 
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_heartbeat_rcvd: Option<DateTime<Utc>>,
    pub ocpp_message_id_counter: u32, 
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct OcppProfileBehavior {
    pub rate_unit: EChargingRateUnit, 
    pub profile_phases_in_ocpp_message: u8, 
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct ChargerElectricalConfig {
    pub nominal_voltage_ln: f32, 
    pub active_phase_count: u8,  
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Default, PartialEq, Eq)]
#[reflect(Serialize, Deserialize, Default)]
pub enum EGunStatusOcpp {
    #[default]
    Available,
    Preparing,
    Charging,
    SuspendedEV,
    SuspendedEVSE,
    Finishing,
    Reserved,
    Unavailable,
    Faulted,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Default)]
#[reflect(Serialize, Deserialize, Default)]
pub struct Gun {
    pub gun_id: u32,
    pub connector_id: u32,
    pub status: EGunStatusOcpp,
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize, Default)]
pub struct Guns(pub Vec<Gun>);
