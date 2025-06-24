// a strongly typed configuration schema for asset templates and instances

use serde::Deserialize;
use crate::common::types::EAssetType;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AssetTemplate {
    pub asset_type: EAssetType,
    #[serde(rename = "components")]
    pub component_configs: Vec<ComponentConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AssetInstance {
    pub external_id: String,
    pub template_id: String,
    pub instance_components: Vec<ComponentConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentConfig {
    AssetInfo { make: String, model: String },
    ChargerElectricalConfig { nominal_voltage_ln: f32, active_phase_count: u8 },
    OcppConfig { version: String, charge_point_id: String },
    OcppProfileBehavior { rate_unit: String, profile_phases_in_ocpp_message: u8 },
    AlfenSpecificConfig { default_tx_profile_power_watts: f32 },
    MeteringSource { source_type: String, details: serde_json::Value },
    ModbusControlConfig { ip: String, port: u16, unit_id: u8 },
}
