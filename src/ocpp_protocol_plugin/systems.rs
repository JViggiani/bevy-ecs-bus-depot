use bevy::prelude::*;
use super::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};
use super::components::{OcppConfig, Guns, ChargerElectricalConfig, OcppProfileBehavior, OcppConnectionState, EGunStatusOcpp};
use crate::core_asset_plugin::{TargetPowerSetpointKw, CurrentMeterReading, MeteringSource, ExternalId, LastAppliedSetpointKw};
use crate::types::{
    EAssetType, EOutgoingOcppMessage, EMeteringDataSource, EOperationalStatus,
    BootNotificationReqPayload, BootNotificationConfPayload, RegistrationStatus,
    StatusNotificationReqPayload, StatusNotificationConfPayload, 
    MeterValuesReqPayload, MeterValuesConfPayload,
    SetChargingProfileReqPayload, CsChargingProfiles, ChargingSchedule, ChargingSchedulePeriod,
};
use chrono::Utc; 

fn map_ocpp_status_to_egunstatus(ocpp_status: &str) -> EGunStatusOcpp {
    match ocpp_status {
        "Available" => EGunStatusOcpp::Available,
        "Preparing" => EGunStatusOcpp::Preparing,
        "Charging" => EGunStatusOcpp::Charging,
        "SuspendedEV" => EGunStatusOcpp::SuspendedEV,
        "SuspendedEVSE" => EGunStatusOcpp::SuspendedEVSE,
        "Finishing" => EGunStatusOcpp::Finishing,
        "Reserved" => EGunStatusOcpp::Reserved,
        "Unavailable" => EGunStatusOcpp::Unavailable,
        "Faulted" => EGunStatusOcpp::Faulted,
        _ => {
            warn!("Unknown OCPP status received: '{}', defaulting to Unavailable.", ocpp_status);
            EGunStatusOcpp::Unavailable
        }
    }
}


pub fn placeholder_ocpp_request_handler_system(
    mut ocpp_requests: EventReader<OcppRequestFromChargerEvent>,
    mut ocpp_commands: EventWriter<SendOcppToChargerCommand>,
    mut charger_query: Query<(
        Entity,
        &ExternalId,
        &OcppConfig,
        &mut OcppConnectionState,
        &mut Guns,
        &mut CurrentMeterReading,
        &MeteringSource,
        &mut crate::types::EOperationalStatus, 
    )>,
) {
    for event in ocpp_requests.read() {
        info!("OCPP Request Handling System: Received for CP_ID '{}', Action '{}', OcppMsgID '{}'. Payload: {:.100}",
            event.charge_point_id,
            event.action,
            event.ocpp_message_id,
            event.payload_json
        );

        let mut charger_entity_id_found: Option<Entity> = None;

        for (entity, _ext_id, ocpp_config, _conn_state, _guns, _meter_reading, _metering_source, _op_status) in charger_query.iter_mut() {
            if ocpp_config.charge_point_id == event.charge_point_id {
                charger_entity_id_found = Some(entity);
                break;
            }
        }

        if let Some(entity_id) = charger_entity_id_found {
            let (_entity, ext_id, ocpp_config, mut conn_state, mut guns, mut meter_reading, metering_source, mut op_status) = 
                charger_query.get_mut(entity_id).unwrap(); 

            info!("Processing OCPP Action '{}' for charger ExtID '{}' (CP_ID '{}')", event.action, ext_id.0, ocpp_config.charge_point_id);

            match event.action.as_str() {
                "BootNotification" => {
                    match serde_json::from_str::<BootNotificationReqPayload>(&event.payload_json) {
                        Ok(payload) => {
                            info!("Parsed BootNotification: Vendor='{}', Model='{}'", payload.charge_point_vendor, payload.charge_point_model);
                            conn_state.is_connected = true;
                            conn_state.last_heartbeat_rcvd = Some(Utc::now());
                            *op_status = EOperationalStatus::Online; 

                            let response_payload = BootNotificationConfPayload {
                                current_time: Utc::now().to_rfc3339(),
                                interval: 300, 
                                status: RegistrationStatus::Accepted,
                            };
                            info!("Sending BootNotification.conf for CP_ID '{}'", ocpp_config.charge_point_id);
                            ocpp_commands.write(SendOcppToChargerCommand {
                                charge_point_id: ocpp_config.charge_point_id.clone(),
                                message_type: EOutgoingOcppMessage::BootNotificationResponse(response_payload),
                                ocpp_message_id: Some(event.ocpp_message_id.clone()), 
                            });
                        }
                        Err(e) => error!("Failed to parse BootNotification payload for CP_ID '{}': {}", ocpp_config.charge_point_id, e),
                    }
                }
                "StatusNotification" => {
                    match serde_json::from_str::<StatusNotificationReqPayload>(&event.payload_json) {
                        Ok(payload) => {
                            info!("Parsed StatusNotification for CP_ID '{}', ConnectorID {}: Status='{}', ErrorCode='{}'", 
                                ocpp_config.charge_point_id, payload.connector_id, payload.status, payload.error_code);
                            
                            let new_gun_status = map_ocpp_status_to_egunstatus(&payload.status);

                            if payload.connector_id == 0 { 
                                if new_gun_status == EGunStatusOcpp::Faulted || payload.error_code != "NoError" {
                                    *op_status = EOperationalStatus::Faulted;
                                    error!("Charger CP_ID '{}' reported fault: ErrorCode='{}', Status='{}'", ocpp_config.charge_point_id, payload.error_code, payload.status);
                                } else if *op_status == EOperationalStatus::Faulted && new_gun_status != EGunStatusOcpp::Faulted {
                                     *op_status = EOperationalStatus::Online; 
                                     info!("Charger CP_ID '{}' recovered from fault.", ocpp_config.charge_point_id);
                                }
                                for gun in guns.0.iter_mut() {
                                   gun.status = new_gun_status.clone(); 
                                }

                            } else { 
                                if let Some(gun) = guns.0.iter_mut().find(|g| g.connector_id == payload.connector_id) {
                                    gun.status = new_gun_status;
                                    info!("Updated Gun {} on CP_ID '{}' to status {:?}", gun.gun_id, ocpp_config.charge_point_id, gun.status);
                                } else {
                                    warn!("Received StatusNotification for unknown ConnectorID {} on CP_ID '{}'", payload.connector_id, ocpp_config.charge_point_id);
                                }
                            }
                            info!("Sending StatusNotification.conf for CP_ID '{}'", ocpp_config.charge_point_id);
                            ocpp_commands.write(SendOcppToChargerCommand {
                                charge_point_id: ocpp_config.charge_point_id.clone(),
                                message_type: EOutgoingOcppMessage::StatusNotificationResponse(StatusNotificationConfPayload{}),
                                ocpp_message_id: Some(event.ocpp_message_id.clone()),
                            });
                        }
                        Err(e) => error!("Failed to parse StatusNotification payload for CP_ID '{}': {}", ocpp_config.charge_point_id, e),
                    }
                }
                "MeterValues" => {
                     match serde_json::from_str::<MeterValuesReqPayload>(&event.payload_json) {
                        Ok(payload) => {
                            info!("Parsed MeterValues for CP_ID '{}', ConnectorID {}: {} samples", 
                                ocpp_config.charge_point_id, payload.connector_id, payload.meter_value.len());
                            if metering_source.source_type == EMeteringDataSource::Ocpp {
                                if let Some(first_sample) = payload.meter_value.first() {
                                    for sv_value in &first_sample.sampled_value {
                                        if sv_value.measurand.as_deref() == Some("Power.Active.Import") {
                                            if let Ok(power_val) = sv_value.value.parse::<f32>() {
                                                let power_kw = if sv_value.unit.as_deref() == Some("kW") { power_val } else { power_val / 1000.0 };
                                                meter_reading.power_kw = power_kw;
                                                info!("Updated MeterReading.power_kw for CP_ID '{}' to {} kW from MeterValues", ocpp_config.charge_point_id, power_kw);
                                            }
                                        }
                                        if sv_value.measurand.as_deref() == Some("Energy.Active.Import.Register") {
                                            if let Ok(energy_val) = sv_value.value.parse::<f64>() {
                                                let energy_kwh = if sv_value.unit.as_deref() == Some("kWh") { energy_val } else { energy_val / 1000.0 };
                                                meter_reading.energy_kwh = energy_kwh;
                                                 info!("Updated MeterReading.energy_kwh for CP_ID '{}' to {} kWh from MeterValues", ocpp_config.charge_point_id, energy_kwh);
                                            }
                                        }
                                    }
                                }
                                meter_reading.timestamp = Utc::now();
                            }
                             info!("Sending MeterValues.conf for CP_ID '{}'", ocpp_config.charge_point_id);
                             ocpp_commands.write(SendOcppToChargerCommand {
                                charge_point_id: ocpp_config.charge_point_id.clone(),
                                message_type: EOutgoingOcppMessage::MeterValuesResponse(MeterValuesConfPayload{}),
                                ocpp_message_id: Some(event.ocpp_message_id.clone()),
                            });
                        }
                        Err(e) => error!("Failed to parse MeterValues payload for CP_ID '{}': {}", ocpp_config.charge_point_id, e),
                    }
                }
                _ => {
                    warn!("Unhandled OCPP Action '{}' for CP_ID '{}'", event.action, ocpp_config.charge_point_id);
                }
            }
        } else {
            warn!("Received OCPP message for unknown charge_point_id: {}", event.charge_point_id);
        }
    }
}

pub fn placeholder_charger_control_to_ocpp_profile_system(
    mut charger_query: Query<(
        &ExternalId,
        &OcppConfig,
        &Guns, 
        &ChargerElectricalConfig,
        &OcppProfileBehavior,
        &TargetPowerSetpointKw,
        &OcppConnectionState,
        &mut LastAppliedSetpointKw,
    ), (With<EAssetType>, Changed<TargetPowerSetpointKw>)>, 
    mut ocpp_commands: EventWriter<SendOcppToChargerCommand>,
    mut id_counter: Local<u32>, 
) {
    for (ext_id, ocpp_cfg, _guns, elec_cfg, profile_behav, target_setpoint, conn_state, mut last_applied_setpoint) in charger_query.iter_mut() {
        if !conn_state.is_connected {
            warn!("Charger {} (ExtID: {}) is not connected. Skipping SetChargingProfile.", ocpp_cfg.charge_point_id, ext_id.0);
            continue;
        }

        if target_setpoint.0 == last_applied_setpoint.0 {
            continue;
        }
        
        info!("TargetPowerSetpointKw for {} (ExtID: {}) changed to {} kW. Current electrical config: {:.1}V L-N, {} active phases. Profile behavior: {:?}, {} phases in OCPP msg.",
            ocpp_cfg.charge_point_id, ext_id.0, target_setpoint.0,
            elec_cfg.nominal_voltage_ln, elec_cfg.active_phase_count,
            profile_behav.rate_unit, profile_behav.profile_phases_in_ocpp_message
        );

        let limit_value: f32;
        let rate_unit_str: String;

        match profile_behav.rate_unit {
            crate::types::EChargingRateUnit::Amps => {
                rate_unit_str = "A".to_string();
                if elec_cfg.nominal_voltage_ln == 0.0 || elec_cfg.active_phase_count == 0 {
                    error!("Invalid electrical config for {} (ExtID: {}): Voltage or phase count is zero. Cannot calculate Amps.", ocpp_cfg.charge_point_id, ext_id.0);
                    continue;
                }
                let current_per_phase = (target_setpoint.0 * 1000.0) / (elec_cfg.nominal_voltage_ln * elec_cfg.active_phase_count as f32);
                limit_value = current_per_phase.max(0.0); 
                info!("Calculated Amps per phase: {:.2} A for {} (ExtID: {})", limit_value, ocpp_cfg.charge_point_id, ext_id.0);
            }
            crate::types::EChargingRateUnit::Watts => {
                rate_unit_str = "W".to_string();
                limit_value = (target_setpoint.0 * 1000.0).max(0.0); 
                info!("Calculated Watts: {:.0} W for {} (ExtID: {})", limit_value, ocpp_cfg.charge_point_id, ext_id.0);
            }
        }
        
        let charging_schedule_period = ChargingSchedulePeriod {
            start_period: 0,
            limit: limit_value,
            number_phases: Some(profile_behav.profile_phases_in_ocpp_message),
        };

        let charging_schedule = ChargingSchedule {
            duration: Some(86400), 
            start_schedule: Some(Utc::now().to_rfc3339()),
            charging_rate_unit: rate_unit_str,
            charging_schedule_period: vec![charging_schedule_period],
            min_charging_rate: Some(0.0),
        };

        let cs_charging_profiles = CsChargingProfiles {
            charging_profile_id: 1, 
            transaction_id: None, 
            stack_level: 1,
            charging_profile_purpose: "TxDefaultProfile".to_string(), 
            charging_profile_kind: "Absolute".to_string(),
            recurrency_kind: Some("Daily".to_string()),
            valid_from: Some(Utc::now().to_rfc3339()),
            valid_to: Some((Utc::now() + chrono::Duration::days(1)).to_rfc3339()),
            charging_schedule,
        };

        let profile_payload = SetChargingProfileReqPayload {
            connector_id: 0, 
            cs_charging_profiles,
        };

        *id_counter += 1;
        let ocpp_message_id = format!("sc_msg_{}", *id_counter);

        info!("Sending SetChargingProfile.req to {} (ExtID: {}), OcppMsgID: {}. Payload: {:?}",
            ocpp_cfg.charge_point_id, ext_id.0, ocpp_message_id, profile_payload);

        ocpp_commands.write(SendOcppToChargerCommand {
            charge_point_id: ocpp_cfg.charge_point_id.clone(),
            message_type: EOutgoingOcppMessage::SetChargingProfileRequest(profile_payload),
            ocpp_message_id: Some(ocpp_message_id), 
        });

        last_applied_setpoint.0 = target_setpoint.0; 
    }
}
