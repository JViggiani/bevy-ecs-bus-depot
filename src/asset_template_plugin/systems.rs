use bevy::prelude::*;
use super::SiteConfigJson;
use serde::Deserialize;
use crate::core_asset_plugin::{ExternalId, AssetInfo, CurrentMeterReading, TargetPowerSetpointKw, MeteringSource, LastAppliedSetpointKw};
use crate::ocpp_protocol_plugin::{OcppConfig, OcppProfileBehavior, ChargerElectricalConfig, Guns, Gun, EGunStatusOcpp, OcppConnectionState, AlfenSpecificConfig, AlfenSpecialInitStatus, GenericChargerInitializationStatus};
use crate::types::{EAssetType, EOperationalStatus};
use crate::modbus_protocol_plugin::ModbusControlConfig;

#[derive(Deserialize)]
struct SiteConfig {
    asset_templates: std::collections::HashMap<String, AssetTemplate>,
    assets: Vec<AssetInstance>,
}

#[derive(Deserialize)]
struct AssetTemplate {
    asset_type: EAssetType,
    components: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct AssetInstance {
    external_id: String,
    template_id: String,
    #[serde(default)]
    instance_components: Vec<serde_json::Value>,
}

fn apply_component(
    commands: &mut Commands,
    entity: Entity,
    component: &serde_json::Value,
    asset_type: EAssetType,
) -> Result<(), String> {
    let component_type = component.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing type field".to_string())?;

    match component_type {
        "AssetInfo" => {
            let data: AssetInfo = serde_json::from_value(component.clone())
                .map_err(|e| format!("AssetInfo: {}", e))?;
            commands.entity(entity).insert(data);
        }
        "ChargerElectricalConfig" => {
            let electrical_config_data: ChargerElectricalConfig = serde_json::from_value(component.clone()).map_err(|e| format!("ChargerElectricalConfig: {}", e))?; // Renamed from data
            commands.entity(entity).insert(electrical_config_data);
        }
        "OcppConfig" => {
            let map = component.as_object().ok_or("OcppConfig: not an object".to_string())?;
            let cp_id = map.get("charge_point_id")
                .and_then(|v| v.as_str())
                .ok_or("OcppConfig: missing charge_point_id".to_string())?
                .to_string();
            let version = serde_json::from_value(
                map.get("version").cloned().unwrap_or(serde_json::Value::String("".into()))
            ).map_err(|e| format!("OcppConfig.version: {}", e))?;
            commands.entity(entity).insert(OcppConfig { charge_point_id: cp_id, version });
        }
         "OcppProfileBehavior" => {
            let profile_behavior_data: OcppProfileBehavior = serde_json::from_value(component.clone()).map_err(|e| format!("OcppProfileBehavior: {}", e))?; // Renamed from data
            commands.entity(entity).insert(profile_behavior_data);
        }
        "AlfenSpecificConfig" => {
            let alfen_config_data: AlfenSpecificConfig = serde_json::from_value(component.clone()).map_err(|e| format!("AlfenSpecificConfig: {}", e))?; // Renamed from data
            commands.entity(entity).insert(alfen_config_data);
            commands.entity(entity).insert(AlfenSpecialInitStatus::default());
        }
        "MeteringSource" => {
            let metering_source_data: MeteringSource = serde_json::from_value(component.clone()).map_err(|e| format!("MeteringSource: {}", e))?; // Renamed from data
            commands.entity(entity).insert(metering_source_data);
        }
        "ModbusControlConfig" => {
             if asset_type == EAssetType::Battery { 
                let modbus_control_data: ModbusControlConfig = serde_json::from_value(component.clone()).map_err(|e| format!("ModbusControlConfig: {}", e))?; // Renamed from data
                commands.entity(entity).insert(modbus_control_data);
            } else {
                warn!("Skipping ModbusControlConfig for non-battery asset type: {:?}", asset_type);
            }
        }
        _ => return Err(format!("Unknown or unhandled component type in config: {}", component_type)),
    }
    Ok(())
}

/// Reads `assets/site_config.json` and spawns entities with configured components.
pub fn spawn_assets_from_config_system(
    mut commands: Commands,
    config_json: Res<SiteConfigJson>,
) {
    let config_text = &config_json.0;
    let site_config: SiteConfig = match serde_json::from_str(config_text) {
        Ok(cfg) => cfg,
        Err(err) => { error!("Invalid JSON in site_config.json: {}", err); return; }
    };

    for instance in site_config.assets {
        let template = match site_config.asset_templates.get(&instance.template_id) {
            Some(t) => t,
            None => {
                warn!("Template '{}' not found for '{}'", instance.template_id, instance.external_id);
                continue;
            }
        };

        // before spawning components:
        let mut entity_commands = commands.spawn_empty();
        let new_entity = entity_commands.id();

        entity_commands.insert((
            ExternalId(instance.external_id.clone()),
            template.asset_type,
            EOperationalStatus::Initializing,
            CurrentMeterReading::default(),
            TargetPowerSetpointKw::default(),
            LastAppliedSetpointKw::default(),
        ));

        if template.asset_type == EAssetType::Charger {
            commands.entity(new_entity).insert((
                Guns(vec![Gun { gun_id: 1, connector_id: 1, status: EGunStatusOcpp::Available, ..Default::default() }]),
                OcppConnectionState::default(),
                GenericChargerInitializationStatus::default(),
            ));
        }

        for comp in &template.components {
            if let Err(err) = apply_component(&mut commands, new_entity, comp, template.asset_type) {
                error!("Error applying template component to '{}': {}", instance.external_id, err);
            }
        }
        for comp in &instance.instance_components {
            if let Err(err) = apply_component(&mut commands, new_entity, comp, template.asset_type) {
                error!("Error applying instance component to '{}': {}", instance.external_id, err);
            }
        }

        info!("Spawned '{}'", instance.external_id);
    }
}
