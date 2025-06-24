use bevy::prelude::*;

use crate::common::types::{EAssetType, EOperationalStatus};

pub mod components;

pub use components::*;

pub struct CoreAssetPlugin;

impl Plugin for CoreAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::ExternalId>()
            .register_type::<components::AssetInfo>()
            .register_type::<EAssetType>()
            .register_type::<EOperationalStatus>()
            .register_type::<components::CurrentMeterReading>()
            .register_type::<components::TargetPowerSetpointKw>()
            .register_type::<components::LastAppliedSetpointKw>()
            .register_type::<components::MeteringSource>();
        
        // Debug‐log setpoint changes only in debug mode
        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_core_assets_system);
    }
}

#[cfg(debug_assertions)]
fn debug_core_assets_system(
    query: Query<(
        Entity,
        &components::ExternalId,
        &components::AssetInfo,
        &EAssetType,
        &components::CurrentMeterReading,
        &components::TargetPowerSetpointKw
    ), Changed<components::TargetPowerSetpointKw>>,
) {
    for (e, id, info, ty, reading, setpoint) in query.iter() {
        debug!(
            "Entity {:?} [{}|{:?}] {} {} → meter: {:?}, setpoint: {} kW",
            e, id.0, ty, info.make, info.model, reading, setpoint.0
        );
    }
}
