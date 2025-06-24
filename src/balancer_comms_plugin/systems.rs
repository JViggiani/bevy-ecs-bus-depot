use bevy::prelude::*;
use super::events::SetpointCommand;
use super::resources::{BalancerSetpointReceiver, BalancerMeteringSender};
use super::balancer_messages::BalancerMeteringMessage;
use crate::core_asset_plugin::{ExternalId, CurrentMeterReading, TargetPowerSetpointKw};
use crate::common::external_id_map::ExternalIdMap;

/// Receives external setpoints and translates them to entity-specific commands
pub fn receive_external_setpoints(
    receiver: Option<Res<BalancerSetpointReceiver>>,
    id_map: Res<ExternalIdMap>,
    mut command_writer: EventWriter<SetpointCommand>,
) {
    let Some(receiver) = receiver else { return };
    
    while let Ok(message) = receiver.0.try_recv() {
        if let Some(&entity) = id_map.0.get(&message.external_id) {
            command_writer.write(SetpointCommand { entity, power_kw: message.target_power_kw });
        } else {
            warn!("No asset for external_id '{}'", message.external_id);
        }
    }
}

/// Applies setpoint commands to their target entities
pub fn apply_setpoint_commands(
    mut commands: EventReader<SetpointCommand>,
    mut query: Query<(&ExternalId, &mut TargetPowerSetpointKw)>,
) {
    for command in commands.read() {
        if let Ok((id, mut setpoint)) = query.get_mut(command.entity) {
            info!("Setting {}kW for '{}'", command.power_kw, id.0);
            setpoint.0 = command.power_kw;
        }
    }
}

/// Exports metering data when it changes
pub fn export_metering_data(
    query: Query<(&ExternalId, &CurrentMeterReading), Changed<CurrentMeterReading>>,
    sender: Option<Res<BalancerMeteringSender>>,
) {
    let Some(sender) = sender else { return };
    
    for (id, reading) in query.iter() {
        let message = BalancerMeteringMessage {
            external_id: id.0.clone(),
            power_kw: reading.power_kw,
            energy_kwh: reading.energy_kwh,
            timestamp: reading.timestamp,
        };
        
        if let Err(e) = sender.0.send(message) {
            error!("Failed to send metering data for '{}': {}", id.0, e);
        }
    }
}