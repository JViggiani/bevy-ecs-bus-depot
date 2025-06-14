use bevy::prelude::*;
pub mod components;
pub mod events;
pub mod systems;

pub use components::*;
pub use events::*;
pub use systems::*;

pub struct ModbusProtocolPlugin;

impl Plugin for ModbusProtocolPlugin {
    fn build(&self, app: &mut App) {
        let (request_tx, _request_rx)   = crossbeam_channel::unbounded::<components::ModbusRequest>();
        let (_response_tx, response_rx) = crossbeam_channel::unbounded::<components::ModbusResponse>();

        app.insert_resource(events::ModbusRequestChannel(request_tx))
           .insert_resource(events::ModbusResponseChannel(response_rx))

           .register_type::<ModbusControlConfig>()
           .register_type::<ModbusAssetLastPoll>()

           // Poll timer fires every 5 seconds
           .insert_resource(systems::ModbusPollTimer(Timer::from_seconds(5.0, TimerMode::Repeating)))
           .add_event::<events::ModbusPollEvent>()

           .add_systems(Update, (
               systems::modbus_poll_timer_system,
               systems::schedule_modbus_requests_on_event,
               systems::process_modbus_responses_system,
               systems::placeholder_modbus_control_system,
           ));

        info!("ModbusProtocolPlugin loaded");
    }
}
