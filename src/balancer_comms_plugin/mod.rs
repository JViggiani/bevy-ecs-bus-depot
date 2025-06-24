use bevy::prelude::*;
use crate::core_asset_plugin::TargetPowerSetpointKw;

pub mod events;
pub mod systems;
pub mod resources;
pub mod balancer_messages;

pub use events::*;
pub use systems::*;
pub use resources::*;
pub use balancer_messages::*;

pub struct BalancerCommsPlugin;

impl Plugin for BalancerCommsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TargetPowerSetpointKw>()
           .add_event::<SetpointCommand>()
           .add_systems(Update, (
               receive_external_setpoints,      // balancer external -> orchestrator internal
               apply_setpoint_commands,         // Process commands
               export_metering_data,            // orchestrator internal -> balancer external
           ));

        info!("BalancerCommsPlugin loaded.");
    }
}