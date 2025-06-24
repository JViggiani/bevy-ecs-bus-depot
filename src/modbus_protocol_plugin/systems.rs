use bevy::prelude::*;
use crate::core_asset_plugin::{ExternalId, CurrentMeterReading, TargetPowerSetpointKw, LastAppliedSetpointKw};
use crate::modbus_protocol_plugin::components::{ModbusControlConfig, ModbusRequest};
use super::{
    ModbusPollEvent, ModbusRequestEvent, ModbusResponseEvent,
    ModbusRequestChannel, ModbusResponseChannel,
};
use crate::core_asset_plugin::MeteringSource;
use crate::core_asset_plugin::components::MeteringSourceDetails;
use crate::common::types::EMeteringDataSource;

/// Timer resource that triggers a Modbus poll every 5 sec.
#[derive(Resource)]
pub struct ModbusPollTimer(pub Timer);

/// Each time the timer finishes, fire a `ModbusPollEvent`.
pub fn modbus_poll_timer_system(
    time: Res<Time>,
    mut timer: ResMut<ModbusPollTimer>,
    mut writer: EventWriter<ModbusPollEvent>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        writer.write(ModbusPollEvent);
    }
}

/// On each `ModbusPollEvent`, enqueue a ModbusRequest for every Modbus‐enabled asset.
pub fn schedule_modbus_requests_on_event(
    mut poll_reader: EventReader<ModbusPollEvent>,
    mut writer: EventWriter<ModbusRequestEvent>,
    query: Query<(Entity, &ExternalId, &MeteringSource)>,
) {
    for _ in poll_reader.read() {
        for (entity, _id, source) in query.iter() {
            if let (
                EMeteringDataSource::Modbus, 
                Some(MeteringSourceDetails::Modbus{ register_map_key, .. })
            ) = (source.source_type, &source.details)
            {
                writer.write(ModbusRequestEvent {
                    entity,
                    register_map_key: register_map_key.clone(),
                });
            }
        }
    }
}

/// Process ModbusResponse from channel and update meter readings.
pub fn ingest_modbus_responses(
    channel: Res<ModbusResponseChannel>,
    mut writer: EventWriter<ModbusResponseEvent>,
) {
    while let Ok(resp) = channel.0.try_recv() {
        writer.write(ModbusResponseEvent {
            external_id: resp.external_id.clone(),
            power_kw: resp.power_kw,
            energy_kwh: resp.energy_kwh,
            timestamp: resp.timestamp,
        });
    }
}

/// Internal request events -> channel send
pub fn send_modbus_requests_to_channel(
    mut reader: EventReader<ModbusRequestEvent>,
    channel: Res<ModbusRequestChannel>,
) {
    for event in reader.read() {
        let _ = channel.0.send(ModbusRequest::new(
            // lookup ExternalId if needed; here we embed ID in request
            event.entity.to_string(),
            event.register_map_key.clone(),
        ));
    }
}

/// 5. Internal response events → component updates
pub fn apply_modbus_responses(
    mut reader: EventReader<ModbusResponseEvent>,
    mut query: Query<(&ExternalId, &mut CurrentMeterReading)>,
) {
    for ev in reader.read() {
        if let Some((_, mut reading)) = query.iter_mut()
            .find(|(id, _)| id.0 == ev.external_id)
        {
            reading.power_kw   = ev.power_kw;
            reading.energy_kwh = ev.energy_kwh;
            reading.timestamp  = ev.timestamp;
        }
    }
}

/// Placeholder control system for Modbus‐controlled assets.
pub fn placeholder_modbus_control_system(
    mut query: Query<(&ExternalId, &TargetPowerSetpointKw, &ModbusControlConfig, &mut LastAppliedSetpointKw), Changed<TargetPowerSetpointKw>>
) {
    for (id, target, cfg, mut last) in query.iter_mut() {
        info!("Modbus Control: {} kW to {}:{} unit {}", target.0, cfg.ip, cfg.port, id.0);
        last.0 = target.0;
    }
}
