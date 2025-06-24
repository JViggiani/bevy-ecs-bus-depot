use bevy::prelude::*;
use bevy_pancam::PanCamPlugin;
use std::collections::HashMap;
use crate::balancer_comms_plugin::balancer_messages::{BalancerMeteringMessage, BalancerSetpointMessage};
use crate::modbus_protocol_plugin::{ModbusRequest, ModbusResponse};
use crate::ocpp_protocol_plugin::events::{OcppCommandToAsset, OcppRequestFromAsset};

pub mod components;
pub mod log_capture;
pub mod resources;
pub mod systems;

pub use components::*;
pub use log_capture::*;
pub use resources::*;
pub use systems::*;

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        let mut templates = MessageTemplateLibrary(HashMap::new());

        templates.0.insert(
            "Balancer Setpoint".to_string(),
            vec![(
                "Setpoint 5kW".to_string(),
                "{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string(),
            )],
        );

        templates.0.insert(
            "OCPP Request from Asset".to_string(),
            vec![
                (
                    "BootNotification".to_string(),
                    "{\n  \"charge_point_id\": \"CH001\",\n  \"action\": \"BootNotification\",\n  \"payload_json\": \"{\\\"charge_point_vendor\\\":\\\"Zenobe\\\",\\\"charge_point_model\\\":\\\"VirtualCharger\\\"}\"\n}".to_string(),
                ),
                (
                    "MeterValues".to_string(),
                    "{\n  \"charge_point_id\": \"CH001\",\n  \"action\": \"MeterValues\",\n  \"payload_json\": \"{\\\"connectorId\\\":1,\\\"meterValue\\\":[{\\\"sampledValue\\\":[{\\\"value\\\":\\\"5000\\\",\\\"measurand\\\":\\\"Power.Active.Import\\\",\\\"unit\\\":\\\"W\\\"}]}]}\"\n}".to_string(),
                ),
            ],
        );

        templates.0.insert(
            "Modbus Response from Asset".to_string(),
            vec![(
                "Active Power Reading".to_string(),
                "{\n  \"external_id\": \"BAT001\",\n  \"power_kw\": 5.0,\n  \"energy_kwh\": 1234.5\n}".to_string(),
            )],
        );

        let default_queue = "OCPP Request from Asset".to_string();
        let default_template_name = "BootNotification".to_string();
        let default_template_json = templates.0.get(&default_queue).unwrap().first().unwrap().1.clone();

        app.insert_resource(PositionsAttached(false))
           .insert_resource(LogMessages::default())
           .insert_resource(OutputMessages::default())
           .insert_resource(systems::OrchestratorSpawned(false))
           .insert_resource(systems::BalancerSpawned(false))
           .insert_resource(templates)
           .insert_resource(SelectedQueue(default_queue))
           .insert_resource(SelectedTemplate(default_template_name))
           .insert_resource(MessageInput(default_template_json))
           .add_plugins(PanCamPlugin::default())
           .add_systems(Startup, setup_camera)
           .add_systems(Update, (
               attach_positions_system.run_if(positions_not_attached),
               spawn_asset_visuals_system.after(attach_positions_system),
               spawn_orchestrator_system.run_if(orchestrator_not_spawned).after(spawn_asset_visuals_system),
               spawn_balancer_system.run_if(balancer_not_spawned).after(spawn_orchestrator_system),
               update_asset_colors_system.after(spawn_balancer_system),
               handle_mouse_clicks_system.after(update_asset_colors_system),
               pull_captured_logs_system.after(handle_mouse_clicks_system),
               pull_output_messages_system.after(pull_captured_logs_system),
               ui_system.after(pull_output_messages_system),
           ));

        info!("VisualizationPlugin loaded");
    }
}

pub fn setup_visualization_channels(
    balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointMessage>,
    ocpp_from_asset_sender: crossbeam_channel::Sender<OcppRequestFromAsset>,
    modbus_response_sender: crossbeam_channel::Sender<ModbusResponse>,
    balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringMessage>,
    ocpp_to_asset_receiver: crossbeam_channel::Receiver<OcppCommandToAsset>,
    modbus_request_receiver: crossbeam_channel::Receiver<ModbusRequest>,
) -> MessageChannels {
    MessageChannels {
        balancer_setpoint_sender,
        ocpp_from_asset_sender,
        modbus_response_sender,
        balancer_metering_receiver,
        ocpp_to_asset_receiver,
        modbus_request_receiver,
    }
}