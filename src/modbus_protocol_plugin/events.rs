use bevy::prelude::*;
use super::components::{ModbusRequest, ModbusResponse};
use chrono::DateTime;

/// Resource holding the sender for ModbusRequest.
#[derive(Resource)]
pub struct ModbusRequestChannel(pub crossbeam_channel::Sender<ModbusRequest>);

/// Resource holding the receiver for ModbusResponse.
#[derive(Resource)]
pub struct ModbusResponseChannel(pub crossbeam_channel::Receiver<ModbusResponse>);

/// Emit this on each timer tick; drives ModbusRequest production.
#[derive(Event)]
pub struct ModbusPollEvent;

/// Internal event for scheduling a Modbus read
#[derive(Event)]
pub struct ModbusRequestEvent {
    pub entity: Entity,
    pub register_map_key: String,
}

/// Internal event carrying an incoming Modbus response
#[derive(Event)]
pub struct ModbusResponseEvent {
    pub external_id: String,
    pub power_kw: f32,
    pub energy_kwh: f64,
    pub timestamp: DateTime<chrono::Utc>,
}
