use bevy::prelude::*;
use bevy::log::LogPlugin;
use crate::core_asset_plugin::CoreAssetPlugin;
use crate::asset_template_plugin::AssetTemplatePlugin;
use crate::ocpp_protocol_plugin::{OcppProtocolPlugin, OcppRequestReceiver, OcppCommandSender};
use crate::modbus_protocol_plugin::{ModbusProtocolPlugin, ModbusRequestChannel, ModbusResponseChannel};
use crate::external_comms_plugin::{ExternalCommsPlugin, IncomingSetpointChannel, OutgoingMeteringChannel};
use crossbeam_channel::{unbounded, Sender, Receiver};

/// External channel ends for production integration or tests.
pub struct AppExternalChannelEnds {
    // balancer ↔ Bevy
    pub balancer_setpoint_sender: Sender<crate::external_comms_plugin::ExternalSetpointData>,
    pub balancer_setpoint_receiver: Receiver<crate::external_comms_plugin::ExternalSetpointData>,
    pub balancer_metering_sender: Sender<crate::external_comms_plugin::ExternalMeteringData>,
    pub balancer_metering_receiver: Receiver<crate::external_comms_plugin::ExternalMeteringData>,

    // Modbus ↔ Bevy
    pub modbus_request_sender: Sender<crate::modbus_protocol_plugin::ModbusRequest>,
    pub modbus_request_receiver: Receiver<crate::modbus_protocol_plugin::ModbusRequest>,
    pub modbus_response_sender: Sender<crate::modbus_protocol_plugin::ModbusResponse>,
    pub modbus_response_receiver: Receiver<crate::modbus_protocol_plugin::ModbusResponse>,

    // OCPP ↔ Bevy
    pub ocpp_request_sender: Sender<crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent>,
    pub ocpp_request_receiver: Receiver<crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent>,
    pub ocpp_command_sender: Sender<crate::ocpp_protocol_plugin::events::SendOcppToChargerCommand>,
    pub ocpp_command_receiver: Receiver<crate::ocpp_protocol_plugin::events::SendOcppToChargerCommand>,
}

pub fn setup_bevy_app() -> (App, AppExternalChannelEnds) {
    let mut app = App::new();

    // Balancer channels
    let (balancer_setpoint_sender, balancer_setpoint_receiver) = unbounded();
    let (balancer_metering_sender, balancer_metering_receiver) = unbounded();

    // Modbus channels
    let (modbus_request_sender, modbus_request_receiver) = unbounded();
    let (modbus_response_sender, modbus_response_receiver) = unbounded();

    // OCPP channels
    let (ocpp_request_sender, ocpp_request_receiver) =
        unbounded::<crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent>();
    let (ocpp_command_sender, ocpp_command_receiver) =
        unbounded::<crate::ocpp_protocol_plugin::events::SendOcppToChargerCommand>();

    app.add_plugins(MinimalPlugins)
       .insert_resource(Time::<Fixed>::from_duration(std::time::Duration::from_secs(5)))
       .add_plugins(LogPlugin::default())
       .add_plugins(CoreAssetPlugin)
       .add_plugins(AssetTemplatePlugin)
       .add_plugins(ExternalCommsPlugin)
       .add_plugins(ModbusProtocolPlugin)
       .add_plugins(OcppProtocolPlugin)

       // insert only the halves needed by ECS/plugin logic:
       .insert_resource(IncomingSetpointChannel(balancer_setpoint_receiver.clone()))
       .insert_resource(OutgoingMeteringChannel(balancer_metering_sender.clone()))
       .insert_resource(ModbusRequestChannel(modbus_request_sender.clone()))
       .insert_resource(ModbusResponseChannel(modbus_response_receiver.clone()))
       .insert_resource(OcppRequestReceiver(ocpp_request_receiver.clone()))
       .insert_resource(OcppCommandSender(ocpp_command_sender.clone()));

    let channels = AppExternalChannelEnds {
        balancer_setpoint_sender,
        balancer_setpoint_receiver,
        balancer_metering_sender,
        balancer_metering_receiver,
        modbus_request_sender,
        modbus_request_receiver,
        modbus_response_sender,
        modbus_response_receiver,
        ocpp_request_sender,
        ocpp_request_receiver,
        ocpp_command_sender,
        ocpp_command_receiver,
    };
    (app, channels)
}
