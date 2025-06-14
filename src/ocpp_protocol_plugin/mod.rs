use bevy::prelude::*;
pub mod components;
pub mod events;
pub mod systems;

pub use components::*;
pub use events::*;
pub use systems::{
    ingest_ocpp_requests_from_channel_system,
    ocpp_request_handler,
    generic_ocpp_charger_initialization_system,
    alfen_special_init_system,
    charger_control_to_ocpp_profile,
    export_ocpp_commands_to_channel_system,
};

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
            .register_type::<AlfenSpecificConfig>() 
            .register_type::<AlfenSpecialInitStatus>() 
            .register_type::<AlfenSpecialInitState>() 
            .register_type::<GenericChargerInitializationStatus>()
            .register_type::<GenericChargerInitProgress>()
            .add_event::<OcppRequestFromChargerEvent>()
            .add_event::<SendOcppToChargerCommand>()
            .add_systems(Update, ingest_ocpp_requests_from_channel_system)
            .add_systems(Update, (
                ocpp_request_handler,
                generic_ocpp_charger_initialization_system.after(ocpp_request_handler),
                alfen_special_init_system.after(generic_ocpp_charger_initialization_system),
                charger_control_to_ocpp_profile.after(alfen_special_init_system),
            ))
            .add_systems(Update, export_ocpp_commands_to_channel_system);
    }
}
