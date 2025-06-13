use bevy::prelude::*;
use serde::Deserialize;
use std::fs;
use crate::core_asset_plugin::{ExternalId, AssetInfo, CurrentMeterReading, TargetPowerSetpointKw, MeteringSource, LastAppliedSetpointKw}; 
use crate::ocpp_protocol_plugin::{OcppConfig, OcppProfileBehavior, ChargerElectricalConfig, Guns, Gun, EGunStatusOcpp, OcppConnectionState};
use crate::types::{EAssetType, EOperationalStatus}; 
use crate::modbus_protocol_plugin::ModbusControlConfig; 
use chrono::Utc;

#[derive(Deserialize, Debug)]
struct AssetInstanceConfig {
    external_id: String,
    template_id: String,
    #[serde(default)]
    instance_components: Vec<serde_json::Value>, 
}

#[derive(Deserialize, Debug)]
struct AssetTemplateConfig {
    asset_type: EAssetType, 
    components: Vec<serde_json::Value>, 
}

#[derive(Deserialize, Debug)]
struct SiteConfig {
    asset_templates: std::collections::HashMap<String, AssetTemplateConfig>,
    assets: Vec<AssetInstanceConfig>,
}

fn apply_component_from_json(
    commands: &mut Commands,
    entity: Entity,
    component_json: &serde_json::Value,
    asset_type_for_defaults: EAssetType, 
) -> Result<(), String> {
    let component_type = component_json.get("type").and_then(|v| v.as_str()).ok_or_else(|| "Component JSON missing 'type' field".to_string())?;

    match component_type {
        "AssetInfo" => {
            let data: AssetInfo = serde_json::from_value(component_json.get("data").unwrap_or(component_json).clone()).map_err(|e| format!("AssetInfo: {}", e))?;
            commands.entity(entity).insert(data);
        }
        "ChargerElectricalConfig" => {
            let data: ChargerElectricalConfig = serde_json::from_value(component_json.get("data").unwrap_or(component_json).clone()).map_err(|e| format!("ChargerElectricalConfig: {}", e))?;
            commands.entity(entity).insert(data);
        }
        "OcppConfig" => {
            let data: OcppConfig = serde_json::from_value(component_json.get("data").unwrap_or(component_json).clone()).map_err(|e| format!("OcppConfig: {}", e))?;
            commands.entity(entity).insert(data);
        }
         "OcppProfileBehavior" => {
            let data: OcppProfileBehavior = serde_json::from_value(component_json.get("data").unwrap_or(component_json).clone()).map_err(|e| format!("OcppProfileBehavior: {}", e))?;
            commands.entity(entity).insert(data);
        }
        "MeteringSource" => {
            let data: MeteringSource = serde_json::from_value(component_json.clone()).map_err(|e| format!("MeteringSource: {}", e))?;
            commands.entity(entity).insert(data);
        }
        "ModbusControlConfig" => {
             if asset_type_for_defaults == EAssetType::Battery { 
                let data: ModbusControlConfig = serde_json::from_value(component_json.get("data").unwrap_or(component_json).clone()).map_err(|e| format!("ModbusControlConfig: {}", e))?;
                commands.entity(entity).insert(data);
            } else {
                warn!("Skipping ModbusControlConfig for non-battery asset type: {:?}", asset_type_for_defaults);
            }
        }
        _ => return Err(format!("Unknown or unhandled component type in config: {}", component_type)),
    }
    Ok(())
}


pub fn spawn_assets_from_config_system(mut commands: Commands) {
    info!("Spawning assets from configuration...");
    let config_path = "assets/site_config.json"; 
    let config_str = match fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to read site_config.json: {}. Ensure it exists at {}.", e, config_path);
            return;
        }
    };

    let site_config: SiteConfig = match serde_json::from_str(&config_str) {
        Ok(sc) => sc,
        Err(e) => {
            error!("Failed to parse site_config.json: {}", e);
            return;
        }
    };

    for asset_instance_config in site_config.assets {
        info!("Processing asset instance: {}", asset_instance_config.external_id);
        let template = match site_config.asset_templates.get(&asset_instance_config.template_id) {
            Some(t) => t,
            None => {
                error!("Template_id '{}' not found for asset '{}'", asset_instance_config.template_id, asset_instance_config.external_id);
                continue;
            }
        };

        let mut entity_commands = commands.spawn_empty();
        let entity_id = entity_commands.id();

        entity_commands.insert((
            ExternalId(asset_instance_config.external_id.clone()),
            template.asset_type, 
            EOperationalStatus::Initializing,
            CurrentMeterReading { timestamp: Utc::now(), ..Default::default() }, 
            TargetPowerSetpointKw::default(),
            LastAppliedSetpointKw::default(), 
        ));
        
        match template.asset_type {
            EAssetType::Charger => {
                entity_commands.insert((
                    Guns(vec![ 
                        Gun { gun_id: 1, connector_id: 1, status: EGunStatusOcpp::Available, ..Default::default()},
                    ]),
                    OcppConnectionState::default(),
                ));
            }
            EAssetType::Battery => {
            }
            _ => {} 
        }

        for component_json_val in &template.components {
            if let Err(e) = apply_component_from_json(&mut commands, entity_id, component_json_val, template.asset_type) {
                error!("Error applying template component for asset {}: {}", asset_instance_config.external_id, e);
            }
        }

        for component_json_val in &asset_instance_config.instance_components {
            if let Err(e) = apply_component_from_json(&mut commands, entity_id, component_json_val, template.asset_type) {
                error!("Error applying instance component for asset {}: {}", asset_instance_config.external_id, e);
            }
        }
        info!("Spawned asset: {} with Entity ID: {:?}", asset_instance_config.external_id, entity_id);
    }
}
