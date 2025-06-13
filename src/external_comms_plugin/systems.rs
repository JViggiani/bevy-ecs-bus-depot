use bevy::prelude::*;
use super::events::IncomingSetpointEvent;
use super::{ExternalMeteringData, IncomingSetpointChannel, OutgoingMeteringChannel};
use crate::core_asset_plugin::{ExternalId, TargetPowerSetpointKw, CurrentMeterReading};
use std::time::SystemTime;


pub fn ingest_setpoints_from_channel_system(
    channel: Res<IncomingSetpointChannel>, 
    mut event_writer: EventWriter<IncomingSetpointEvent>,
) {
    while let Ok(data) = channel.0.try_recv() {
        info!("ExternalComms: Ingested setpoint from channel for ExtID '{}': {} kW", data.external_id, data.target_power_kw);
        event_writer.write(IncomingSetpointEvent {
            external_id: data.external_id,
            target_power_kw: data.target_power_kw,
        });
    }
}

pub fn apply_incoming_setpoints_system(
    mut events: EventReader<IncomingSetpointEvent>,
    mut query: Query<(&ExternalId, &mut TargetPowerSetpointKw)>,
) {
    for event in events.read() {
        let mut found_asset = false;
        for (external_id_comp, mut target_setpoint_comp) in query.iter_mut() {
            if external_id_comp.0 == event.external_id {
                info!("ExternalComms: Applying incoming setpoint to asset ExtID '{}': Target {} kW", event.external_id, event.target_power_kw);
                target_setpoint_comp.0 = event.target_power_kw;
                found_asset = true;
                break; 
            }
        }
        if !found_asset {
            warn!("ExternalComms: Received setpoint for unknown ExtID '{}'", event.external_id);
        }
    }
}

pub fn export_metering_data_to_channel_system(
    query: Query<(&ExternalId, &CurrentMeterReading), Changed<CurrentMeterReading>>,
    channel: Res<OutgoingMeteringChannel>, 
) {
    for (external_id_comp, meter_reading_comp) in query.iter() {
        let reading_system_time: SystemTime = meter_reading_comp.timestamp.into();

        let data = ExternalMeteringData {
            external_id: external_id_comp.0.clone(),
            power_kw: meter_reading_comp.power_kw,
            energy_kwh: meter_reading_comp.energy_kwh,
            timestamp: reading_system_time,
        };
        if let Err(e) = channel.0.send(data.clone()) {
            error!("ExternalComms: Failed to send metering data for ExtID '{}' to channel: {}", external_id_comp.0, e);
        } else {
            info!("ExternalComms: Exported metering data for ExtID '{}': {:.2} kW, {:.2} kWh, Timestamp {:?}", 
                external_id_comp.0, data.power_kw, data.energy_kwh, data.timestamp);
        }
    }
}
