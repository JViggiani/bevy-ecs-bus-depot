use bevy::prelude::*;
use std::collections::HashMap;
use crate::balancer_comms_plugin::balancer_messages::{BalancerSetpointMessage, BalancerMeteringMessage};
use crate::modbus_protocol_plugin::{ModbusRequest, ModbusResponse};
use crate::ocpp_protocol_plugin::events::{OcppRequestFromAsset, OcppCommandToAsset};

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
    pub balancer_setpoint_sender:   crossbeam_channel::Sender<BalancerSetpointMessage>,
    pub ocpp_from_asset_sender:     crossbeam_channel::Sender<OcppRequestFromAsset>,
    pub modbus_response_sender:     crossbeam_channel::Sender<ModbusResponse>,
    pub balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringMessage>,
    pub ocpp_to_asset_receiver:     crossbeam_channel::Receiver<OcppCommandToAsset>,
    pub modbus_request_receiver:    crossbeam_channel::Receiver<ModbusRequest>,
}

#[derive(Resource)]
pub struct MessageTemplateLibrary(pub HashMap<String, Vec<(String, String)>>);

#[derive(Resource, Default)]
pub struct SelectedQueue(pub String);

#[derive(Resource, Default)]
pub struct SelectedTemplate(pub String);

#[derive(Resource, Default)]
pub struct MessageInput(pub String);
