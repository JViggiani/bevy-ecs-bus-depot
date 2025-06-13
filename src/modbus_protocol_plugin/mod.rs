use bevy::prelude::*;
pub mod components;
pub mod systems; 

pub use components::*;
pub use systems::*; 

pub struct ModbusProtocolPlugin;

impl Plugin for ModbusProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ModbusControlConfig>()
            .register_type::<ModbusAssetLastPoll>()
            .add_systems(Update, (
                placeholder_modbus_poll_system,
                placeholder_modbus_control_system,
            ));
        info!("ModbusProtocolPlugin loaded with placeholder poll and control systems.");
    }
}
