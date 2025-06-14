use bevy::prelude::*;
pub mod components;

pub use components::*;

pub struct CoreAssetPlugin;

impl Plugin for CoreAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ExternalId>()
            .register_type::<AssetInfo>()
            .register_type::<crate::types::EAssetType>()
            .register_type::<crate::types::EOperationalStatus>()
            .register_type::<CurrentMeterReading>()
            .register_type::<TargetPowerSetpointKw>()
            .register_type::<LastAppliedSetpointKw>()
            .register_type::<MeteringSource>()
            .add_systems(Update, (
                debug_core_assets_system,
            ));
    }
}

fn debug_core_assets_system(
    changed_setpoint_asset_query: Query<(Entity, &ExternalId, &AssetInfo, &crate::types::EAssetType, &CurrentMeterReading, &TargetPowerSetpointKw), Changed<TargetPowerSetpointKw>>
) {
    for (entity_id, external_id_component, asset_info_component, asset_type_component, meter_reading_component, target_setpoint_component) in changed_setpoint_asset_query.iter() {
        debug!(
            "Entity {:?}, ID: {}, Type: {:?}, Make: {}, Model: {}, Meter: {:?}, New Setpoint: {} kW",
            entity_id, external_id_component.0, asset_type_component, asset_info_component.make, asset_info_component.model, meter_reading_component, target_setpoint_component.0
        );
    }
}
