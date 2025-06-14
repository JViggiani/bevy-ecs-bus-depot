use bevy::prelude::*;
use super::components::{ModbusRequest, ModbusResponse};

/// Resource holding the sender for ModbusRequest.
#[derive(Resource)]
pub struct ModbusRequestChannel(pub crossbeam_channel::Sender<ModbusRequest>);

/// Resource holding the receiver for ModbusResponse.
#[derive(Resource)]
pub struct ModbusResponseChannel(pub crossbeam_channel::Receiver<ModbusResponse>);

/// Emit this on each timer tick; drives ModbusRequest production.
#[derive(Event)]
pub struct ModbusPollEvent;
