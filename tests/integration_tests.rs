use bevy::prelude::*;
use ocpp_bevy_poc::app_setup::{setup_bevy_app, AppMode};
use ocpp_bevy_poc::balancer_comms_plugin::BalancerSetpointData;
use ocpp_bevy_poc::ocpp_protocol_plugin::events::{
    OcppRequestFromAsset,
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

    // 1. Provide minimal site config
    let site_config_json = r#"{
        "asset_templates": {
            "Phihong_AC_EU_Charger_Template": {
                "asset_type": "Charger",
                "components": [
                    { "type": "AssetInfo", "make": "Phihong", "model": "AC_EU_Dual_V2" },
                    { "type": "ChargerElectricalConfig", "nominal_voltage_ln": 230.0, "active_phase_count": 3 },
                    { "type": "OcppProfileBehavior", "rate_unit": "Amps", "profile_phases_in_ocpp_message": 3 },
                    { "type": "MeteringSource", "source_type": "Ocpp", "details": { "Ocpp": {} } }
                ]
            }
        },
        "assets": [
            {
                "external_id": "CH001",
                "template_id": "Phihong_AC_EU_Charger_Template",
                "instance_components": [
                    { "type": "OcppConfig", "version": "V1_6J", "charge_point_id": "CH001" }
                ]
            }
        ]
    }"#.to_string();

    // 2. Standard app setup with custom config
    let (mut bevy_app, channels) = setup_bevy_app(site_config_json, AppMode::Headless, None);

    // 3. Grab OCPP and balancer channels
    let ocpp_from_asset_sender    = channels.ocpp_from_asset_sender.clone();
    let ocpp_to_asset_receiver    = &channels.ocpp_to_asset_receiver;
    let balancer_setpoint_sender  = &channels.balancer_setpoint_sender;

    // let the ECS startup systems (asset spawn, init) run once
    bevy_app.update();

    // 4. Simulate BootNotification
    let boot_notification = BootNotificationReqPayload {
        charge_point_vendor: "TestVendor".into(),
        charge_point_model:  "TestModel".into(),
    };
    ocpp_from_asset_sender.send(OcppRequestFromAsset {
        charge_point_id: asset_external_id.clone(),
        action:          "BootNotification".into(),
        payload_json:    serde_json::to_string(&boot_notification).unwrap(),
        ocpp_message_id: "1".into(),
    }).unwrap();

    // Update app called twice:
    //  once to pull from the OcppRequestReceiver and fire the ingest_ocpp_requests_from_channel_system
    //  again to let ocpp_request_handler respond and export_ocpp_commands_to_channel_system to push the response onto ocpp_command_receiver 
    bevy_app.update(); 
    bevy_app.update();

    let boot_response = try_recv(ocpp_to_asset_receiver, Duration::from_secs(2)).unwrap();
    assert_eq!(boot_response.charge_point_id, asset_external_id.clone());
    assert_eq!(boot_response.ocpp_message_id, Some("1".into()));
    if let EOutgoingOcppMessage::BootNotificationResponse(conf) = boot_response.message_type {
        assert_eq!(conf.status, RegistrationStatus::Accepted);
    } else {
        panic!("Expected BootNotificationResponse");
    }

    // 5. Drain generic‐init commands
    for _ in 0..10 {
        if try_recv(ocpp_to_asset_receiver, Duration::from_millis(50)).is_none() {
            break;
        }
    }

    // 6. Simulate StatusNotification
    let status_notification = StatusNotificationReqPayload {
        connector_id: 1,
        error_code:   "NoError".into(),
        status:       "Available".into(),
    };
    ocpp_from_asset_sender.send(OcppRequestFromAsset {
        charge_point_id: asset_external_id.clone(),
        action:          "StatusNotification".into(),
        payload_json:    serde_json::to_string(&status_notification).unwrap(),
        ocpp_message_id: "2".into(),
    }).unwrap();

    // Similar to before we need to call update twice
    bevy_app.update(); bevy_app.update();

    let status_response = try_recv(ocpp_to_asset_receiver, Duration::from_secs(1)).unwrap();
    assert_eq!(status_response.charge_point_id, asset_external_id.clone());
    assert_eq!(status_response.ocpp_message_id, Some("2".into()));
    if !matches!(status_response.message_type, EOutgoingOcppMessage::StatusNotificationResponse(_)) {
        panic!("Expected StatusNotificationResponse");
    }

    // 7. Send 10 kW setpoint
    balancer_setpoint_sender.send(BalancerSetpointData {
        external_id:     asset_external_id.clone(),
        target_power_kw: 10.0,
    }).unwrap();

    // Call update three times to ensure all systems (including export_ocpp_commands_to_channel_system) run
    bevy_app.update(); bevy_app.update(); bevy_app.update();

    let profile10 = try_recv(ocpp_to_asset_receiver, Duration::from_secs(1));
    assert!(profile10.is_some(), "Expected SetChargingProfileRequest command, but none was received.");

    let profile10 = profile10.unwrap();
    assert_eq!(profile10.charge_point_id, asset_external_id.clone());
    if let EOutgoingOcppMessage::SetChargingProfileRequest(req) = profile10.message_type {
        // The profile behavior is configured for "Amps", so we assert the unit is "A"
        assert_eq!(req.cs_charging_profiles.charging_schedule.charging_rate_unit, "A");
        let limit = req.cs_charging_profiles.charging_schedule.charging_schedule_period[0].limit;
        // The limit is now in Amps: (10000W) / (230V * 3 phases) = 14.49A
        assert!((limit - 14.49).abs() < 0.1, "Limit was {}", limit);
    } else {
        panic!("Expected SetChargingProfileRequest for 10 kW");
    }

    // 8. Send 5 kW setpoint
    balancer_setpoint_sender.send(BalancerSetpointData {
        external_id:     asset_external_id.clone(),
        target_power_kw: 5.0,
    }).unwrap();
    bevy_app.update(); bevy_app.update(); bevy_app.update();

    let profile5 = try_recv(ocpp_to_asset_receiver, Duration::from_secs(1));
    assert!(profile5.is_some(), "Expected SetChargingProfileRequest command, but none was received.");

    let profile5 = profile5.unwrap();
    assert_eq!(profile5.charge_point_id, asset_external_id);
    if let EOutgoingOcppMessage::SetChargingProfileRequest(req) = profile5.message_type {
        // The profile behavior is configured for "Amps", so we assert the unit is "A"
        assert_eq!(req.cs_charging_profiles.charging_schedule.charging_rate_unit, "A");
        let limit = req.cs_charging_profiles.charging_schedule.charging_schedule_period[0].limit;
        // The limit is now in Amps: (5000W) / (230V * 3 phases) = 7.24A
        assert!((limit - 7.24).abs() < 0.1, "Limit was {}", limit);
    } else {
        panic!("Expected SetChargingProfileRequest for 5 kW");
    }
}
