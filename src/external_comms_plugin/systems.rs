use bevy::prelude::*;
use super::events::IncomingSetpointEvent;
use super::{ExternalMeteringData, IncomingSetpointChannel, OutgoingMeteringChannel};
use crate::core_asset_plugin::{ExternalId, TargetPowerSetpointKw, CurrentMeterReading};
use std::time::SystemTime;

/// Pulls incoming setpoints from the channel and emits a Bevy event.
pub fn ingest_setpoints_from_channel_system(
    incoming: Res<IncomingSetpointChannel>,
    mut writer: EventWriter<IncomingSetpointEvent>,
) {
    while let Ok(data) = incoming.0.try_recv() {
        writer.write(IncomingSetpointEvent {
            external_id: data.external_id,
            target_power_kw: data.target_power_kw,
        });
    }
}

/// Applies incoming setpoint events to matching assets.
pub fn apply_incoming_setpoints_system(
    mut reader: EventReader<IncomingSetpointEvent>,
    mut query: Query<(&ExternalId, &mut TargetPowerSetpointKw)>,
) {
    for event in reader.read() {
        if let Some((_, mut setpoint)) = query.iter_mut()
            .find(|(id, _)| id.0 == event.external_id)
        {
            info!("Setting {} kW to '{}'", event.target_power_kw, event.external_id);
            setpoint.0 = event.target_power_kw;
        } else {
            warn!("No asset with ID '{}'", event.external_id);
        }
    }
}

/// Sends updated metering readings out over the channel.
pub fn export_metering_data_to_channel_system(
    query: Query<(&ExternalId, &CurrentMeterReading), Changed<CurrentMeterReading>>,
    outgoing: Res<OutgoingMeteringChannel>,
) {
    for (id, reading) in query.iter() {
        let data = ExternalMeteringData {
            external_id: id.0.clone(),
            power_kw: reading.power_kw,
            energy_kwh: reading.energy_kwh,
            timestamp: SystemTime::now(),
        };
        if let Err(err) = outgoing.0.send(data.clone()) {
            error!("Failed to send metering data for '{}': {}", id.0, err);
        }
    }
}
