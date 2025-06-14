use bevy::prelude::*;
use ocpp_bevy_poc::app_setup::setup_bevy_app;
use ocpp_bevy_poc::external_comms_plugin::ExternalSetpointData;
use ocpp_bevy_poc::ocpp_protocol_plugin::events::{
    OcppRequestFromChargerEvent,
};
use ocpp_bevy_poc::types::{
    BootNotificationReqPayload,
    StatusNotificationReqPayload,
    EOutgoingOcppMessage,
    RegistrationStatus,
};
use crossbeam_channel::Receiver;
use std::time::Duration;
use std::thread;

fn try_recv<T>(rx: &Receiver<T>, timeout: Duration) -> Option<T> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Ok(v) = rx.try_recv() {
            return Some(v);
        }
        thread::sleep(Duration::from_millis(10));
    }
    None
}

#[test]
fn test_charger_connect_setpoint_update() {
    let asset_external_id = "CH001".to_string();

    // 1. Standard app setup
    let (mut bevy_app, channels) = setup_bevy_app();

    // 2. Grab OCPP and balancer channels
    let charger_request_sender   = channels.ocpp_request_sender.clone();
    let command_rx               = &channels.ocpp_command_receiver;
    let balancer_setpoint_sender = &channels.balancer_setpoint_sender;

    // let the ECS startup systems (asset spawn, init) run once
    bevy_app.update();

    // 3. Simulate BootNotification
    let boot_notification = BootNotificationReqPayload {
        charge_point_vendor: "TestVendor".into(),
        charge_point_model:  "TestModel".into(),
    };
    charger_request_sender.send(OcppRequestFromChargerEvent {
        charge_point_id: asset_external_id.clone(),
        action:          "BootNotification".into(),
        payload_json:    serde_json::to_string(&boot_notification).unwrap(),
        ocpp_message_id: "1".into(),
    }).unwrap();
    bevy_app.update(); bevy_app.update();

    let boot_response = try_recv(command_rx, Duration::from_secs(2)).unwrap();
    assert_eq!(boot_response.charge_point_id, asset_external_id.clone());
    assert_eq!(boot_response.ocpp_message_id, Some("1".into()));
    if let EOutgoingOcppMessage::BootNotificationResponse(conf) = boot_response.message_type {
        assert_eq!(conf.status, RegistrationStatus::Accepted);
    } else {
        panic!("Expected BootNotificationResponse");
    }

    // 4. Drain generic‐init commands
    for _ in 0..10 {
        if try_recv(command_rx, Duration::from_millis(50)).is_none() {
            break;
        }
    }

    // 5. Simulate StatusNotification
    let status_notification = StatusNotificationReqPayload {
        connector_id: 1,
        error_code:   "NoError".into(),
        status:       "Available".into(),
    };
    charger_request_sender.send(OcppRequestFromChargerEvent {
        charge_point_id: asset_external_id.clone(),
        action:          "StatusNotification".into(),
        payload_json:    serde_json::to_string(&status_notification).unwrap(),
        ocpp_message_id: "2".into(),
    }).unwrap();
    bevy_app.update(); bevy_app.update();

    let status_response = try_recv(command_rx, Duration::from_secs(1)).unwrap();
    assert_eq!(status_response.charge_point_id, asset_external_id.clone());
    assert_eq!(status_response.ocpp_message_id, Some("2".into()));
    if !matches!(status_response.message_type, EOutgoingOcppMessage::StatusNotificationResponse(_)) {
        panic!("Expected StatusNotificationResponse");
    }

    // 6. Send 10 kW setpoint
    balancer_setpoint_sender.send(ExternalSetpointData {
        external_id:     asset_external_id.clone(),
        target_power_kw: 10.0,
    }).unwrap();
    bevy_app.update(); bevy_app.update();

    let profile10 = try_recv(command_rx, Duration::from_secs(1)).unwrap();
    assert_eq!(profile10.charge_point_id, asset_external_id.clone());
    if let EOutgoingOcppMessage::SetChargingProfileRequest(req) = profile10.message_type {
        let limit = req.cs_charging_profiles.charging_schedule.charging_schedule_period[0].limit;
        assert!((limit - 14.49).abs() < 0.1);
    } else {
        panic!("Expected SetChargingProfileRequest for 10 kW");
    }

    // 7. Send 5 kW setpoint
    balancer_setpoint_sender.send(ExternalSetpointData {
        external_id:     asset_external_id.clone(),
        target_power_kw: 5.0,
    }).unwrap();
    bevy_app.update(); bevy_app.update();

    let profile5 = try_recv(command_rx, Duration::from_secs(1)).unwrap();
    assert_eq!(profile5.charge_point_id, asset_external_id);
    if let EOutgoingOcppMessage::SetChargingProfileRequest(req) = profile5.message_type {
        let limit = req.cs_charging_profiles.charging_schedule.charging_schedule_period[0].limit;
        assert!((limit - 7.24).abs() < 0.1);
    } else {
        panic!("Expected SetChargingProfileRequest for 5 kW");
    }
}
