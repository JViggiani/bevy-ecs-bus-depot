use bevy::prelude::*;
use bevy_pancam::PanCamPlugin;

pub mod components;
pub mod log_capture;
pub mod systems;

use crate::balancer_comms_plugin::{BalancerSetpointData, BalancerMeteringData};
use crate::modbus_protocol_plugin::{ModbusRequest, ModbusResponse};
use crate::ocpp_protocol_plugin::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};
use self::systems::*;

#[derive(Resource, Default)]
pub struct PositionsAttached(pub bool);

#[derive(Resource, Default)]
pub struct LogMessages(pub Vec<String>);

#[derive(Resource, Default)]
pub struct OutputMessages {
    pub balancer_metering: Vec<String>,
    pub ocpp_commands: Vec<String>,
    pub modbus_requests: Vec<String>,
}

#[derive(Resource)]
pub struct MessageChannels {
    // Senders for input
    pub balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    pub ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
    pub modbus_response_sender: crossbeam_channel::Sender<ModbusResponse>,
    // Receivers for output
    pub balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringData>,
    pub ocpp_command_receiver: crossbeam_channel::Receiver<SendOcppToChargerCommand>,
    pub modbus_request_receiver: crossbeam_channel::Receiver<ModbusRequest>,
}

#[derive(Resource, Default)]
pub struct SelectedQueue(pub String);

#[derive(Resource, Default)]
pub struct MessageInput(pub String);

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PositionsAttached(false))
           .insert_resource(LogMessages::default())
           .insert_resource(OutputMessages::default())
           .insert_resource(SelectedQueue("Balancer Setpoint".to_string()))
           .insert_resource(MessageInput("{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string()))
           .add_plugins(PanCamPlugin::default())
           .add_systems(Startup, setup_camera)
           .add_systems(Update, (
               attach_positions_system.run_if(positions_not_attached),
               spawn_asset_visuals_system,
               spawn_orchestrator_system.run_if(orchestrator_not_spawned),
               spawn_balancer_system.run_if(balancer_not_spawned),
               update_asset_colors_system,
               handle_mouse_clicks_system,
           ))
           .add_systems(Update, (
               pull_captured_logs_system,
               pull_output_messages_system,
               ui_system,
           ));
    }
}

pub fn setup_visualization_channels(
    balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
    modbus_response_sender: crossbeam_channel::Sender<ModbusResponse>,
    balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringData>,
    ocpp_command_receiver: crossbeam_channel::Receiver<SendOcppToChargerCommand>,
    modbus_request_receiver: crossbeam_channel::Receiver<ModbusRequest>,
) -> MessageChannels {
    MessageChannels {
        balancer_setpoint_sender,
        ocpp_request_sender,
        modbus_response_sender,
        balancer_metering_receiver,
        ocpp_command_receiver,
        modbus_request_receiver,
    }
}