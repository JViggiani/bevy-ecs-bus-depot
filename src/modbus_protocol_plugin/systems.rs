use bevy::prelude::*;
use crate::core_asset_plugin::{ExternalId, CurrentMeterReading, TargetPowerSetpointKw, MeteringSource, MeteringSourceDetails, LastAppliedSetpointKw};
use super::components::{ModbusControlConfig, ModbusAssetLastPoll};
use chrono::Utc;

pub fn placeholder_modbus_poll_system(
    mut commands: Commands,
    mut query: Query<(Entity, &ExternalId, &mut CurrentMeterReading, &MeteringSource, Option<&mut ModbusAssetLastPoll>)>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();
    for (entity, ext_id, mut meter_reading, metering_source, modbus_last_poll_opt) in query.iter_mut() {
        if let MeteringSource {
            source_type: crate::types::EMeteringDataSource::Modbus,
            details: Some(MeteringSourceDetails::Modbus { ip, port, unit_id, poll_interval_ms, register_map_key })
        } = metering_source {
            let mut last_poll_component = match modbus_last_poll_opt {
                Some(comp) => comp,
                None => {
                    commands.entity(entity).insert(ModbusAssetLastPoll(0.0));
                    continue; 
                }
            };

            let interval_seconds = *poll_interval_ms as f32 / 1000.0;

            if current_time - last_poll_component.0 >= interval_seconds {
                info!("Modbus Poll System: Polling asset ExtID '{}' at IP '{}', Port {}, Unit ID {}.",
                    ext_id.0, ip, port, unit_id);
                info!("Modbus Poll System: Using register map key '{}'.", register_map_key);

                if ext_id.0 == "BAT001" {
                    meter_reading.power_kw += 0.1;
                    if meter_reading.power_kw > 50.0 { meter_reading.power_kw = -50.0; }
                    let energy_increment = (meter_reading.power_kw as f64 * (interval_seconds as f64 / 3600.0)).abs();
                    meter_reading.energy_kwh += energy_increment;
                } else {
                     meter_reading.power_kw = 1.23;
                     meter_reading.energy_kwh += 0.01;
                }
                meter_reading.timestamp = Utc::now();

                info!("Modbus Poll System: Simulated data for ExtID '{}'. Updated CurrentMeterReading: Power {:.2} kW, Energy {:.2} kWh.",
                    ext_id.0, meter_reading.power_kw, meter_reading.energy_kwh);

                last_poll_component.0 = current_time;
            }
        }
    }
}

pub fn placeholder_modbus_control_system(
    mut query: Query<(Entity, &ExternalId, &TargetPowerSetpointKw, &ModbusControlConfig, &mut LastAppliedSetpointKw), Changed<TargetPowerSetpointKw>>,
) {
    for (_entity, ext_id, target_setpoint, modbus_control_cfg, mut last_applied_setpoint) in query.iter_mut() {
        info!("Modbus Control System: TargetPowerSetpointKw for ExtID '{}' changed to {} kW.",
            ext_id.0, target_setpoint.0);
        info!("Modbus Control System: Asset uses ModbusControlConfig: IP '{}', Port {}, Unit ID {}.",
            modbus_control_cfg.ip, modbus_control_cfg.port, modbus_control_cfg.unit_id);

        info!("Modbus Control System: Would write {:.2} kW to appropriate Modbus register for ExtID '{}'.",
            target_setpoint.0, ext_id.0);
        
        last_applied_setpoint.0 = target_setpoint.0;
        info!("Modbus Control System: Updated LastAppliedSetpointKw for ExtID '{}' to {:.2} kW.", ext_id.0, last_applied_setpoint.0);
    }
}
