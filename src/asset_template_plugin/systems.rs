use bevy::prelude::*;
use crate::asset_template_plugin::{SiteConfig, TotalAssets};
use crate::core_asset_plugin::{ExternalId, AssetInfo, CurrentMeterReading, TargetPowerSetpointKw, LastAppliedSetpointKw, MeteringSource};
use crate::ocpp_protocol_plugin::{OcppConfig, OcppProfileBehavior, ChargerElectricalConfig, Guns, Gun, EGunStatusOcpp, OcppConnectionState, AlfenSpecificConfig, GenericChargerInitializationStatus, AlfenSpecialInitStatus};
use crate::modbus_protocol_plugin::ModbusControlConfig;
use crate::common::types::{EAssetType, EOperationalStatus};
use super::config::ComponentConfig;
use crate::common::external_id_map::ExternalIdMap;

fn apply_component(
    commands: &mut Commands,
    entity: Entity,
    cfg: &ComponentConfig,
    asset_type: EAssetType,
) {
    match cfg {
        ComponentConfig::AssetInfo { make, model } => {
            commands.entity(entity).insert(AssetInfo { make: make.clone(), model: model.clone() });
        }
        ComponentConfig::ChargerElectricalConfig { nominal_voltage_ln, active_phase_count } => {
            commands.entity(entity).insert(ChargerElectricalConfig { nominal_voltage_ln: *nominal_voltage_ln, active_phase_count: *active_phase_count });
        }
        ComponentConfig::OcppConfig { version, charge_point_id } => {
            commands.entity(entity).insert(OcppConfig { charge_point_id: charge_point_id.clone(), version: version.parse().unwrap() });
        }
        ComponentConfig::OcppProfileBehavior { rate_unit, profile_phases_in_ocpp_message } => {
            commands.entity(entity).insert(OcppProfileBehavior {
                rate_unit: rate_unit.parse().unwrap(),
                profile_phases_in_ocpp_message: *profile_phases_in_ocpp_message,
            });
        }
        ComponentConfig::AlfenSpecificConfig { default_tx_profile_power_watts } => {
            commands.entity(entity)
                .insert(AlfenSpecificConfig { default_tx_profile_power_watts: *default_tx_profile_power_watts })
                .insert(AlfenSpecialInitStatus::default());
        }
        ComponentConfig::MeteringSource { source_type, details } => {
            commands.entity(entity).insert(MeteringSource {
                source_type: source_type.parse().unwrap(),
                details: Some(serde_json::from_value(details.clone()).unwrap()),
            });
        }
        ComponentConfig::ModbusControlConfig { ip, port, unit_id } if asset_type == EAssetType::Battery => {
            commands.entity(entity).insert(ModbusControlConfig { ip: ip.clone(), port: *port, unit_id: *unit_id });
        }
        // Skip irrelevant or mis-typed entries
        _ => (),
    }
}

pub fn spawn_assets_from_config_system(
    mut commands: Commands,
    mut id_map: ResMut<ExternalIdMap>,
    config: Res<SiteConfig>,
    mut total_assets: ResMut<TotalAssets>,
) {
    total_assets.0 = config.assets.len();

    for instance in &config.assets {
        let template = match config.asset_templates.get(&instance.template_id) {
            Some(t) => t,
            None => {
                warn!("Missing template '{}'", instance.template_id);
                continue;
            }
        };

        let entity = commands.spawn_empty()
            .insert((
                ExternalId(instance.external_id.clone()),
                template.asset_type,
                CurrentMeterReading::default(),
                TargetPowerSetpointKw::default(),
                LastAppliedSetpointKw::default(),
                EOperationalStatus::default(),
            ))
            .id();

        // record the mapping once:
        id_map.0.insert(instance.external_id.clone(), entity);

        // Charger-specific defaults
        if template.asset_type == EAssetType::Charger {
            commands.entity(entity).insert((
                Guns(vec![Gun { gun_id: 1, connector_id: 1, status: EGunStatusOcpp::Available, ..Default::default() }]),
                OcppConnectionState::default(),
                GenericChargerInitializationStatus::default(),
            ));
        }

        // Apply both template and instance components
        for cfg in template.component_configs.iter().chain(&instance.instance_components) {
            apply_component(&mut commands, entity, cfg, template.asset_type);
        }
        info!("Spawned '{}'", instance.external_id);
    }
}