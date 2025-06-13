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
    query: Query<(Entity, &ExternalId, &AssetInfo, &crate::types::EAssetType, &CurrentMeterReading, &TargetPowerSetpointKw), Changed<TargetPowerSetpointKw>>
) {
    for (entity, ext_id, info, asset_type, meter, setpoint) in query.iter() {
        info!(
            "CoreAsset Debug: Entity {:?}, ID: {}, Type: {:?}, Make: {}, Model: {}, Meter: {:?}, New Setpoint: {} kW",
            entity, ext_id.0, asset_type, info.make, info.model, meter, setpoint.0
        );
    }
}
