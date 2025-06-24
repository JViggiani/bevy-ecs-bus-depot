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
        app.register_type::<ModbusControlConfig>()
           .register_type::<ModbusAssetLastPoll>()
           .insert_resource(ModbusPollTimer(Timer::from_seconds(5.0, TimerMode::Repeating)))
           .add_event::<ModbusPollEvent>()
           .add_event::<ModbusRequestEvent>()
           .add_event::<ModbusResponseEvent>()
           .add_systems(Update, (
               systems::modbus_poll_timer_system,
               systems::schedule_modbus_requests_on_event.after(systems::modbus_poll_timer_system),
               systems::send_modbus_requests_to_channel.after(systems::schedule_modbus_requests_on_event),
               systems::ingest_modbus_responses.after(systems::send_modbus_requests_to_channel),
               systems::apply_modbus_responses.after(systems::ingest_modbus_responses),
               systems::placeholder_modbus_control_system.after(systems::apply_modbus_responses),
           ));
        info!("ModbusProtocolPlugin loaded");
    }
}
