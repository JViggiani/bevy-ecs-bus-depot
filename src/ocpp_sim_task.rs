use bevy::prelude::*;
use crossbeam_channel::{unbounded, Sender, Receiver};
use crate::ocpp_protocol_plugin::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};
use crate::ocpp_protocol_plugin::components::OcppConfig;
use crate::types::{BootNotificationReqPayload, StatusNotificationReqPayload, MeterValuesReqPayload, MeterSample, MeterValueSampledValue, EOutgoingOcppMessage}; 
use std::{time::Duration, collections::HashMap};
use chrono::Utc;

#[derive(Resource)]
pub struct OcppSimEventReceiverForBevy(Receiver<OcppRequestFromChargerEvent>);

#[derive(Resource, Default)]
pub struct OcppSimSpecificCommandSenders(pub HashMap<String, Sender<SendOcppToChargerCommand>>);

pub fn forward_bevy_commands_to_sim_thread(
    mut commands_to_sim: EventReader<SendOcppToChargerCommand>,
    sim_command_senders: Res<OcppSimSpecificCommandSenders>,
) {
    for command in commands_to_sim.read() {
        if let Some(sender) = sim_command_senders.0.get(&command.charge_point_id) {
            if let Err(e) = sender.send(command.clone()) {
                error!("Failed to send command to OCPP sim thread for CP_ID '{}': {}", command.charge_point_id, e);
            }
        } else {
            warn!("No OCPP simulator command channel found for CP_ID '{}'", command.charge_point_id);
        }
    }
}

fn run_single_ocpp_client_simulation(
    charge_point_id_sim: String,
    sim_event_sender_to_bevy: Sender<OcppRequestFromChargerEvent>,
    sim_command_receiver_for_specific_sim: Receiver<SendOcppToChargerCommand>,
) {
    info!("OCPP Simulator Thread Started for CP_ID: {}.", charge_point_id_sim);
    let mut ocpp_message_id_counter = 0;
    let mut current_set_power_kw: Option<f32> = None;

    ocpp_message_id_counter += 1;
    let boot_payload = BootNotificationReqPayload {
        charge_point_model: "SimCharger3000".to_string(),
        charge_point_vendor: "BevySim Inc.".to_string(),
    };
    if let Err(e) = sim_event_sender_to_bevy.send(OcppRequestFromChargerEvent {
        charge_point_id: charge_point_id_sim.clone(),
        ocpp_message_id: ocpp_message_id_counter.to_string(),
        action: "BootNotification".to_string(),
        payload_json: serde_json::to_string(&boot_payload).unwrap(),
    }) {
        error!("OCPP Sim ({}): Failed to send BootNotification event: {}", charge_point_id_sim, e);
        return;
    }
    info!("OCPP Sim ({}): Sent BootNotification.req", charge_point_id_sim);

    ocpp_message_id_counter += 1;
    let status_payload = StatusNotificationReqPayload {
        connector_id: 1,
        error_code: "NoError".to_string(),
        status: "Available".to_string(),
    };
    if let Err(e) = sim_event_sender_to_bevy.send(OcppRequestFromChargerEvent {
        charge_point_id: charge_point_id_sim.clone(),
        ocpp_message_id: ocpp_message_id_counter.to_string(),
        action: "StatusNotification".to_string(),
        payload_json: serde_json::to_string(&status_payload).unwrap(),
    }) {
        error!("OCPP Sim ({}): Failed to send StatusNotification event: {}", charge_point_id_sim, e);
        return;
    }
    info!("OCPP Sim ({}): Sent StatusNotification.req for connector 1", charge_point_id_sim);

    loop {
        match sim_command_receiver_for_specific_sim.try_recv() {
            Ok(command) => {
                info!(
                    "OCPP Sim ({}): Received command, OCPP MsgID: {:?}, Action: {:?}",
                    charge_point_id_sim,
                    command.ocpp_message_id,
                    command.message_type
                );

                match command.message_type {
                    EOutgoingOcppMessage::SetChargingProfileRequest(payload) => {
                        info!("OCPP Sim ({}): Processing SetChargingProfile for transaction ID: {:?}, Profile ID: {}", charge_point_id_sim, payload.cs_charging_profiles.transaction_id, payload.cs_charging_profiles.charging_profile_id);
                        if let Some(period) = payload.cs_charging_profiles.charging_schedule.charging_schedule_period.first() {
                            let limit = period.limit;
                            let unit = &payload.cs_charging_profiles.charging_schedule.charging_rate_unit;
                            if unit == "W" {
                                current_set_power_kw = Some(limit / 1000.0);
                                info!("OCPP Sim ({}): Stored setpoint: {:.2} kW (from Watts)", charge_point_id_sim, current_set_power_kw.unwrap_or(0.0));
                            } else if unit == "A" {
                                current_set_power_kw = Some(0.0);
                                info!("OCPP Sim ({}): Setpoint received in Amps ({:.1} A). Reporting 0.0 kW as sim cannot convert.", charge_point_id_sim, limit);
                            } else {
                                current_set_power_kw = None;
                                info!("OCPP Sim ({}): Setpoint received with unknown unit '{}'. Reporting 0.0 kW.", charge_point_id_sim, unit);
                            }
                        }
                    }
                    EOutgoingOcppMessage::RemoteStartTransactionRequest(payload) => {
                         info!("OCPP Sim ({}): Processing RemoteStartTransactionRequest for idTag: {}", charge_point_id_sim, payload.id_tag);
                    }
                    _ => {
                        info!("OCPP Sim ({}): Received other command type: {:?}", charge_point_id_sim, command.message_type);
                    }
                }
            }
            Err(crossbeam_channel::TryRecvError::Empty) => { }
            Err(e) => {
                error!("OCPP Sim ({}): Error receiving command: {}. Exiting sim thread.", charge_point_id_sim, e);
                break;
            }
        }

        std::thread::sleep(Duration::from_secs(60));
        ocpp_message_id_counter += 1;
        let power_to_report_kw = current_set_power_kw.unwrap_or(0.0);

        let meter_values_payload = MeterValuesReqPayload {
            connector_id: 1,
            transaction_id: None,
            meter_value: vec![
                MeterSample {
                    timestamp: Some(Utc::now().to_rfc3339()),
                    sampled_value: vec![
                        MeterValueSampledValue {
                            value: "1234.5".to_string(),
                            context: Some("Sample.Periodic".to_string()),
                            measurand: Some("Energy.Active.Import.Register".to_string()),
                            unit: Some("Wh".to_string()),
                            format: None, location: None, phase: None,
                        },
                        MeterValueSampledValue {
                            value: format!("{:.1}", power_to_report_kw),
                            context: Some("Sample.Periodic".to_string()),
                            measurand: Some("Power.Active.Import".to_string()),
                            unit: Some("kW".to_string()),
                            format: None, location: None, phase: None,
                        },
                    ],
                }
            ]
        };
        if let Err(e) = sim_event_sender_to_bevy.send(OcppRequestFromChargerEvent {
            charge_point_id: charge_point_id_sim.clone(),
            ocpp_message_id: ocpp_message_id_counter.to_string(),
            action: "MeterValues".to_string(),
            payload_json: serde_json::to_string(&meter_values_payload).unwrap(),
        }) {
            error!("OCPP Sim ({}): Failed to send MeterValues event: {}. Exiting sim thread.", charge_point_id_sim, e);
            break;
        }
        info!("OCPP Sim ({}): Sent MeterValues.req (Power: {} kW)", charge_point_id_sim, power_to_report_kw);
    }
}

pub fn setup_ocpp_simulation(
    mut commands: Commands,
    ocpp_chargers_query: Query<&OcppConfig>,
    mut sim_specific_senders: ResMut<OcppSimSpecificCommandSenders>,
) {
    let (sim_event_sender_to_bevy, sim_event_receiver_for_bevy) = unbounded::<OcppRequestFromChargerEvent>();
    commands.insert_resource(OcppSimEventReceiverForBevy(sim_event_receiver_for_bevy));

    for ocpp_config in ocpp_chargers_query.iter() {
        let charge_point_id = ocpp_config.charge_point_id.clone();
        
        let (specific_command_sender, specific_command_receiver) = unbounded::<SendOcppToChargerCommand>();
        
        sim_specific_senders.0.insert(charge_point_id.clone(), specific_command_sender);

        let event_sender_clone = sim_event_sender_to_bevy.clone();
        
        std::thread::spawn(move || {
            run_single_ocpp_client_simulation(
                charge_point_id,
                event_sender_clone,
                specific_command_receiver,
            );
        });
        info!("Launched OCPP simulation thread for CP_ID: {}", ocpp_config.charge_point_id);
    }
    if ocpp_chargers_query.iter().count() == 0 {
        info!("No OCPP chargers found to simulate.");
    }
}

pub fn poll_ocpp_sim_events_and_fire_bevy_events(
    receiver: Res<OcppSimEventReceiverForBevy>,
    mut ocpp_event_writer: EventWriter<OcppRequestFromChargerEvent>,
) {
    while let Ok(event) = receiver.0.try_recv() {
        info!("OCPP Sim Event Poller: Firing Bevy event for CP_ID '{}', Action '{}'", event.charge_point_id, event.action);
        ocpp_event_writer.write(event);
    }
}
