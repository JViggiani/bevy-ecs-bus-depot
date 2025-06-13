use bevy::prelude::*;
pub mod components;
pub mod events;
pub mod systems;

pub use components::*;
pub use events::*;
pub use systems::*;


pub struct OcppProtocolPlugin;

impl Plugin for OcppProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<OcppConfig>()
            .register_type::<OcppConnectionState>()
            .register_type::<OcppProfileBehavior>()
            .register_type::<ChargerElectricalConfig>()
            .register_type::<EGunStatusOcpp>()
            .register_type::<Gun>()
            .register_type::<Guns>()
            .add_event::<OcppRequestFromChargerEvent>()
            .add_event::<SendOcppToChargerCommand>()
            .add_systems(Update, (
                placeholder_ocpp_request_handler_system,
                placeholder_charger_control_to_ocpp_profile_system,
            ).chain());
    }
}
