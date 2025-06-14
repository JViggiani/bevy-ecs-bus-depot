use bevy::prelude::*;
use super::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};
use super::components::*;
use crate::core_asset_plugin::{TargetPowerSetpointKw, CurrentMeterReading, MeteringSource, ExternalId, LastAppliedSetpointKw};
use crate::types::*;
use chrono::Utc;
use crate::ocpp_protocol_plugin::events::{OcppRequestReceiver, OcppCommandSender};
use crossbeam_channel::TryRecvError;

/// Translate OCPP status string to internal enum.
fn map_status_to_gun_status(status: &str) -> EGunStatusOcpp {
    match status {
        "Available"       => EGunStatusOcpp::Available,
        "Preparing"       => EGunStatusOcpp::Preparing,
        "Charging"        => EGunStatusOcpp::Charging,
        "SuspendedEV"     => EGunStatusOcpp::SuspendedEV,
        "SuspendedEVSE"   => EGunStatusOcpp::SuspendedEVSE,
        "Finishing"       => EGunStatusOcpp::Finishing,
        "Reserved"        => EGunStatusOcpp::Reserved,
        "Unavailable"     => EGunStatusOcpp::Unavailable,
        "Faulted"         => EGunStatusOcpp::Faulted,
        other             => {
            warn!("Unknown OCPP status '{}', defaulting to Unavailable.", other);
            EGunStatusOcpp::Unavailable
        }
    }
}

/// Handle incoming OCPP requests (BootNotification, StatusNotification, MeterValues).
pub fn ocpp_request_handler(
    mut event_reader: EventReader<OcppRequestFromChargerEvent>,
    mut command_writer: EventWriter<SendOcppToChargerCommand>,
    mut charger_query: Query<(
        Entity,
        &ExternalId,
        &mut OcppConnectionState,
        &mut Guns,
        &mut CurrentMeterReading,
        &MeteringSource,
        &mut EOperationalStatus,
    )>,
) {
    for request in event_reader.read() {
        info!("Received {} for '{}'", request.action, request.charge_point_id);

        // Find matching charger entity by ExternalId
        if let Some(entity) = charger_query.iter_mut()
            .find(|(_, ext_id, _, _, _, _, _)| ext_id.0 == request.charge_point_id)
            .map(|(e, _, _, _, _, _, _)| e)
        {
            let (_e, external_id, mut conn, mut guns, mut reading, source, mut status) =
                charger_query.get_mut(entity).unwrap();
            let cp_id = external_id.0.clone();

            match request.action.as_str() {
                "BootNotification" => {
                    if let Ok(_payload) = serde_json::from_str::<BootNotificationReqPayload>(&request.payload_json) {
                        conn.is_connected = true;
                        conn.last_heartbeat_rcvd = Some(Utc::now());
                        *status = EOperationalStatus::Online;

                        let response = BootNotificationConfPayload {
                            current_time: Utc::now().to_rfc3339(),
                            interval:     300,
                            status:       RegistrationStatus::Accepted,
                        };
                        command_writer.write(SendOcppToChargerCommand {
                            charge_point_id: cp_id.clone(),
                            message_type:    EOutgoingOcppMessage::BootNotificationResponse(response),
                            ocpp_message_id: Some(request.ocpp_message_id.clone()),
                        });
                    } else {
                        error!("Invalid BootNotification payload");
                    }
                }

                "StatusNotification" => {
                    if let Ok(payload) = serde_json::from_str::<StatusNotificationReqPayload>(&request.payload_json) {
                        let status_enum = map_status_to_gun_status(&payload.status);

                        if payload.connector_id == 0 {
                            if status_enum == EGunStatusOcpp::Faulted || payload.error_code != "NoError" {
                                *status = EOperationalStatus::Faulted;
                            } else if *status == EOperationalStatus::Faulted {
                                *status = EOperationalStatus::Online;
                            }
                            for gun in guns.0.iter_mut() {
                                gun.status = status_enum.clone();
                            }
                        } else if let Some(gun) = guns.0.iter_mut().find(|g| g.connector_id == payload.connector_id) {
                            gun.status = status_enum;
                        } else {
                            warn!("Unknown connector {}", payload.connector_id);
                        }

                        command_writer.write(SendOcppToChargerCommand {
                            charge_point_id: cp_id.clone(),
                            message_type:    EOutgoingOcppMessage::StatusNotificationResponse(StatusNotificationConfPayload {}),
                            ocpp_message_id: Some(request.ocpp_message_id.clone()),
                        });
                    } else {
                        error!("Invalid StatusNotification payload");
                    }
                }

                "MeterValues" => {
                    if let Ok(payload) = serde_json::from_str::<MeterValuesReqPayload>(&request.payload_json) {
                        if source.source_type == EMeteringDataSource::Ocpp {
                            if let Some(sample) = payload.meter_value.first() {
                                for sv in &sample.sampled_value {
                                    match sv.measurand.as_deref() {
                                        Some("Power.Active.Import") => {
                                            if let Ok(val) = sv.value.parse::<f32>() {
                                                reading.power_kw = if sv.unit.as_deref() == Some("kW") { val } else { val / 1000.0 };
                                            }
                                        }
                                        Some("Energy.Active.Import.Register") => {
                                            if let Ok(val) = sv.value.parse::<f64>() {
                                                reading.energy_kwh = if sv.unit.as_deref() == Some("kWh") { val } else { val / 1000.0 };
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                reading.timestamp = Utc::now();
                            }
                        }
                        command_writer.write(SendOcppToChargerCommand {
                            charge_point_id: cp_id.clone(),
                            message_type:    EOutgoingOcppMessage::MeterValuesResponse(MeterValuesConfPayload {}),
                            ocpp_message_id: Some(request.ocpp_message_id.clone()),
                        });
                    } else {
                        error!("Invalid MeterValues payload");
                    }
                }

                other => warn!("Unhandled OCPP action '{}'", other),
            }
        } else {
            warn!("No charger found for '{}'", request.charge_point_id);
        }
    }
}

/// Send SetChargingProfile requests when target power changes.
pub fn charger_control_to_ocpp_profile(
    mut query: Query<(
        &ExternalId,
        &OcppConfig,
        &Guns,
        &ChargerElectricalConfig,
        &OcppProfileBehavior,
        &TargetPowerSetpointKw,
        &OcppConnectionState,
        &mut LastAppliedSetpointKw,
    ), (With<EAssetType>, Changed<TargetPowerSetpointKw>)>,
    mut command_writer: EventWriter<SendOcppToChargerCommand>,
    mut message_id_counter: Local<u32>,
) {
    for (_external_id, config, _guns, elec_cfg, behavior, target_kw, conn, mut last_kw) in query.iter_mut() {
        if target_kw.0 == last_kw.0 || !conn.is_connected {
            continue;
        }
        *message_id_counter += 1;
        let msg_id = format!("sc_{}", *message_id_counter);
        let cp_id = config.charge_point_id.clone();

        let limit = match behavior.rate_unit {
            EChargingRateUnit::Amps => {
                let per_phase = (target_kw.0 * 1000.0)
                    / (elec_cfg.nominal_voltage_ln * elec_cfg.active_phase_count as f32);
                per_phase.max(0.0)
            }
            EChargingRateUnit::Watts => (target_kw.0 * 1000.0).max(0.0),
        };

        let schedule = ChargingSchedule {
            duration:              Some(86400),
            start_schedule:        Some(Utc::now().to_rfc3339()),
            charging_rate_unit:    if behavior.rate_unit == EChargingRateUnit::Amps { "A" } else { "W" }.to_string(),
            charging_schedule_period: vec![ChargingSchedulePeriod {
                start_period:    0,
                limit,
                number_phases:   Some(behavior.profile_phases_in_ocpp_message),
            }],
            min_charging_rate:     Some(0.0),
        };

        let profiles = CsChargingProfiles {
            charging_profile_id:    1,
            transaction_id:         None,
            stack_level:            1,
            charging_profile_purpose: "TxDefaultProfile".to_string(),
            charging_profile_kind:  "Absolute".to_string(),
            recurrency_kind:        Some("Daily".to_string()),
            valid_from:             Some(Utc::now().to_rfc3339()),
            valid_to:               Some((Utc::now() + chrono::Duration::days(1)).to_rfc3339()),
            charging_schedule:      schedule,
        };

        command_writer.write(SendOcppToChargerCommand {
            charge_point_id: cp_id.clone(),
            message_type:    EOutgoingOcppMessage::SetChargingProfileRequest(SetChargingProfileReqPayload {
                connector_id: 0,
                cs_charging_profiles: profiles,
            }),
            ocpp_message_id: Some(msg_id),
        });

        last_kw.0 = target_kw.0;
    }
}

/// Send OCPP command with unique message ID.
fn send_ocpp_command_helper(
    target_charge_point_id: &str,
    ocpp_message_to_send: EOutgoingOcppMessage,
    ocpp_command_event_writer_ref: &mut EventWriter<SendOcppToChargerCommand>,
    ocpp_message_id_counter_value: &mut u32,
    message_id_prefix: &str,
) {
    *ocpp_message_id_counter_value += 1;
    let ocpp_message_id_string_generated = format!("{}_{}", message_id_prefix, *ocpp_message_id_counter_value);
    ocpp_command_event_writer_ref.write(SendOcppToChargerCommand {
        charge_point_id: target_charge_point_id.to_string(),
        message_type: ocpp_message_to_send,
        ocpp_message_id: Some(ocpp_message_id_string_generated),
    });
}

/// Generic initialization for OCPP chargers.
pub fn generic_ocpp_charger_initialization_system(
    mut generic_chargers_query: Query<(
        &ExternalId,
        &OcppConfig,
        &OcppConnectionState,
        &Guns,
        &ChargerElectricalConfig,
        &mut GenericChargerInitializationStatus,
    ), Without<AlfenSpecificConfig>>,
    mut alfen_chargers_init_query: Query<(
        &ExternalId,
        &OcppConfig,
        &OcppConnectionState,
        &Guns,
        &ChargerElectricalConfig,
        &mut GenericChargerInitializationStatus,
        &AlfenSpecificConfig,
    )>,
    mut ocpp_command_writer: EventWriter<SendOcppToChargerCommand>,
    mut ocpp_message_id_generator_local: Local<u32>,
) {
    let ocpp_message_id_generator_ref_mut = &mut *ocpp_message_id_generator_local;

    let mut process_charger_for_initialization = | 
        external_id_comp: &ExternalId,
        ocpp_config_comp: &OcppConfig,
        connection_state_comp: &OcppConnectionState,
        guns_comp: &Guns,
        _electrical_config_comp: &ChargerElectricalConfig,
        mut initialization_status_comp: Mut<GenericChargerInitializationStatus>
    | {
        if connection_state_comp.is_connected && initialization_status_comp.0 == GenericChargerInitProgress::Pending {
            info!("Generic OCPP Init for {} (ExtID: {}): Starting", ocpp_config_comp.charge_point_id, external_id_comp.0);

            send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::ChangeConfigurationRequest(ChangeConfigurationReqPayload {
                key: "HeartbeatInterval".to_string(), value: "300".to_string()
            }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "generic_init");
            send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::ChangeConfigurationRequest(ChangeConfigurationReqPayload {
                key: "MeterValueSampleInterval".to_string(), value: "60".to_string()
            }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "generic_init");
            send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::ChangeConfigurationRequest(ChangeConfigurationReqPayload {
                key: "LocalAuthorizeOffline".to_string(), value: "true".to_string()
            }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "generic_init");

            for gun_configuration_item in guns_comp.0.iter() {
                let clear_profile_data = CsChargingProfiles {
                    charging_profile_id: gun_configuration_item.connector_id as i32,
                    stack_level: 0,
                    charging_profile_purpose: "TxDefaultProfile".to_string(),
                    charging_profile_kind: "Recurring".to_string(),
                    charging_schedule: ChargingSchedule {
                        charging_rate_unit: "W".to_string(),
                        charging_schedule_period: vec![ChargingSchedulePeriod{start_period:0, limit: 0.0, number_phases: Some(0)}],
                        ..Default::default()
                    },
                    ..Default::default()
                };
                send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::SetChargingProfileRequest(SetChargingProfileReqPayload {
                    connector_id: gun_configuration_item.connector_id,
                    cs_charging_profiles: clear_profile_data,
                }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "generic_clear_txdef");


                let default_tx_profile_data = CsChargingProfiles {
                    charging_profile_id: gun_configuration_item.connector_id as i32,
                    transaction_id: None,
                    stack_level: 1,
                    charging_profile_purpose: "TxDefaultProfile".to_string(),
                    charging_profile_kind: "Recurring".to_string(),
                    recurrency_kind: Some("Daily".to_string()),
                    charging_schedule: ChargingSchedule {
                        duration: Some(86400),
                        start_schedule: Some("00:00:00".to_string()),
                        charging_rate_unit: "W".to_string(),
                        charging_schedule_period: vec![ChargingSchedulePeriod {
                            start_period: 0,
                            limit: 0.0, // Default to 0W, actual power set by balancer
                            number_phases: Some(0), // Let charger decide or use connector 0 default
                        }],
                        min_charging_rate: Some(0.0),
                    },
                    ..Default::default()
                };
                 send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::SetChargingProfileRequest(SetChargingProfileReqPayload {
                    connector_id: gun_configuration_item.connector_id,
                    cs_charging_profiles: default_tx_profile_data,
                }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "generic_init_txdef");
            }

            initialization_status_comp.0 = GenericChargerInitProgress::Complete;
            info!("Generic OCPP Init for {} (ExtID: {}): Sequence sent.", ocpp_config_comp.charge_point_id, external_id_comp.0);
        }
    };

    for (ext_id_val, ocpp_cfg_val, conn_state_val, guns_val, elec_cfg_val, init_status_val) in generic_chargers_query.iter_mut() {
        process_charger_for_initialization(ext_id_val, ocpp_cfg_val, conn_state_val, guns_val, elec_cfg_val, init_status_val);
    }
    for (ext_id_val, ocpp_cfg_val, conn_state_val, guns_val, elec_cfg_val, init_status_val, _alfen_cfg_val) in alfen_chargers_init_query.iter_mut() {
        process_charger_for_initialization(ext_id_val, ocpp_cfg_val, conn_state_val, guns_val, elec_cfg_val, init_status_val);
    }
}

/// Alfen-specific initialization sequence.
pub fn alfen_special_init_system(
    mut alfen_chargers_special_init_query: Query<(
        &ExternalId,
        &OcppConfig,
        &ChargerElectricalConfig,
        &Guns,
        &AlfenSpecificConfig,
        &GenericChargerInitializationStatus,
        &mut AlfenSpecialInitStatus,
    )>,
    mut ocpp_command_writer: EventWriter<SendOcppToChargerCommand>,
    mut ocpp_message_id_generator_local: Local<u32>,
) {
    let ocpp_message_id_generator_ref_mut = &mut *ocpp_message_id_generator_local;

    for (external_id_comp, ocpp_config_comp, electrical_config_comp, guns_comp, alfen_specific_config_comp, generic_init_status_comp, mut alfen_init_status_comp) in alfen_chargers_special_init_query.iter_mut() {
        if generic_init_status_comp.0 == GenericChargerInitProgress::Complete && alfen_init_status_comp.0 == AlfenSpecialInitState::Pending {
            info!("Alfen Charger {} (ExtID: {}) generic init complete. Performing special Alfen initialization.", ocpp_config_comp.charge_point_id, external_id_comp.0);
            alfen_init_status_comp.0 = AlfenSpecialInitState::InProgress;

            send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::ChangeConfigurationRequest(ChangeConfigurationReqPayload {
                key: "MeterValuesSampledData".to_string(),
                value: "Power.Active.Import,Current.Offered,Energy.Active.Import.Register,Current.Import,Voltage".to_string(),
            }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "alfen_init");
            info!("Alfen Init ({}): Sent ChangeConfiguration for MeterValuesSampledData", ocpp_config_comp.charge_point_id);

            send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::ChangeConfigurationRequest(ChangeConfigurationReqPayload {
                key: "WebSocketPingInterval".to_string(),
                value: "60".to_string(),
            }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "alfen_init");
            info!("Alfen Init ({}): Sent ChangeConfiguration for WebSocketPingInterval", ocpp_config_comp.charge_point_id);

            for gun_configuration_item in guns_comp.0.iter() {
                let schedule_period_data = ChargingSchedulePeriod {
                    start_period: 0,
                    limit: alfen_specific_config_comp.default_tx_profile_power_watts,
                    number_phases: Some(electrical_config_comp.active_phase_count.max(1)),
                };

                let schedule_data = ChargingSchedule {
                    duration: None,
                    start_schedule: None,
                    charging_rate_unit: "W".to_string(),
                    charging_schedule_period: vec![schedule_period_data],
                    min_charging_rate: Some(0.0),
                };

                let charging_profiles_data = CsChargingProfiles {
                    charging_profile_id: gun_configuration_item.connector_id as i32 * 100 + 2,
                    transaction_id: None,
                    stack_level: gun_configuration_item.connector_id as u32 * 10 + 2,
                    charging_profile_purpose: "TxDefaultProfile".to_string(),
                    charging_profile_kind: "Relative".to_string(),
                    recurrency_kind: None,
                    valid_from: None,
                    valid_to: None,
                    charging_schedule: schedule_data,
                };
                
                send_ocpp_command_helper(&ocpp_config_comp.charge_point_id, EOutgoingOcppMessage::SetChargingProfileRequest(SetChargingProfileReqPayload {
                    connector_id: gun_configuration_item.connector_id,
                    cs_charging_profiles: charging_profiles_data,
                }), &mut ocpp_command_writer, ocpp_message_id_generator_ref_mut, "alfen_txdef");
                info!("Alfen Init ({}): Sent SetChargingProfile (TxDefaultProfile, Relative) for Connector ID {} with {}W, {} phases",
                    ocpp_config_comp.charge_point_id, gun_configuration_item.connector_id, alfen_specific_config_comp.default_tx_profile_power_watts, electrical_config_comp.active_phase_count.max(1));
            }
            
            alfen_init_status_comp.0 = AlfenSpecialInitState::Complete;
            info!("Alfen Charger {} (ExtID: {}) special initialization sequence sent.", ocpp_config_comp.charge_point_id, external_id_comp.0);
        }
    }
}

/// Pull raw OCPP requests from the channel resource and fire Bevy events.
pub fn ingest_ocpp_requests_from_channel_system(
    channel: Res<OcppRequestReceiver>,
    mut writer: EventWriter<OcppRequestFromChargerEvent>,
) {
    loop {
        match channel.0.try_recv() {
            Ok(request) => { writer.write(request); }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

/// Drain Bevy‐generated `SendOcppToChargerCommand` events and push them into the channel resource.
pub fn export_ocpp_commands_to_channel_system(
    mut reader: EventReader<SendOcppToChargerCommand>,
    channel: Res<OcppCommandSender>,
) {
    for cmd in reader.read() {
        let _ = channel.0.send(cmd.clone());
    }
}
