use bevy::prelude::*;
use crate::core_asset_plugin::{ExternalId, TargetPowerSetpointKw, CurrentMeterReading, MeteringSource, MeteringSourceDetails, LastAppliedSetpointKw};
use crate::modbus_protocol_plugin::ModbusControlConfig;
use super::components::ModbusRequest;
use super::events::{ModbusPollEvent, ModbusRequestChannel, ModbusResponseChannel};

/// Timer resource that triggers a Modbus poll every 5 sec.
#[derive(Resource)]
pub struct ModbusPollTimer(pub Timer);

/// Each time the timer finishes, fire a `ModbusPollEvent`.
pub fn modbus_poll_timer_system(
    time: Res<Time>,
    mut timer: ResMut<ModbusPollTimer>,
    mut poll_event_writer: EventWriter<ModbusPollEvent>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        poll_event_writer.write(ModbusPollEvent);
    }
}

/// On each `ModbusPollEvent`, enqueue a ModbusRequest for every Modbus‐enabled asset.
pub fn schedule_modbus_requests_on_event(
    mut poll_reader: EventReader<ModbusPollEvent>,
    request_channel: Res<ModbusRequestChannel>,
    query: Query<(&ExternalId, &MeteringSource)>,
) {
    for _ in poll_reader.read() {
        for (id, source) in query.iter() {
            if let MeteringSource {
                source_type: crate::types::EMeteringDataSource::Modbus,
                details: Some(MeteringSourceDetails::Modbus { register_map_key, .. })
            } = source {
                let _ = request_channel.0.send(ModbusRequest::new(
                    id.0.clone(),
                    register_map_key.clone(),
                ));
            }
        }
    }
}

/// Process ModbusResponse from channel and update meter readings.
pub fn process_modbus_responses_system(
    response_channel: Res<ModbusResponseChannel>,
    mut query: Query<(&ExternalId, &mut CurrentMeterReading)>,
) {
    while let Ok(response) = response_channel.0.try_recv() {
        if let Some((_, mut reading)) = query.iter_mut()
            .find(|(id, _)| id.0 == response.external_id)
        {
            reading.power_kw   = response.power_kw;
            reading.energy_kwh = response.energy_kwh;
            reading.timestamp  = response.timestamp;
        }
    }
}

/// Placeholder control system for Modbus‐controlled assets.
pub fn placeholder_modbus_control_system(
    mut query: Query<(&ExternalId, &TargetPowerSetpointKw, &ModbusControlConfig, &mut LastAppliedSetpointKw), Changed<TargetPowerSetpointKw>>,
) {
    for (id, target, config, mut last) in query.iter_mut() {
        info!("Modbus Control: {} kW to {}:{} unit {}", target.0, config.ip, config.port, id.0);
        last.0 = target.0;
    }
}
